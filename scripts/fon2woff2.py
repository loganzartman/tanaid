#!/usr/bin/env python3
"""Convert strikes of a Windows .fon bitmap font into unhinted vector woff2 files.

Every pixel becomes exact square geometry (traced into real contours, so holes
and diagonals stay correct), and one em is set to one character cell, so
`font-size: <cell>px` renders a strike at its native 1:1 size.

No hinting is produced: no fpgm/prep/cvt tables, no glyph instructions, and a
gasp table that asks for grayscale with grid-fitting off.

Strikes are selected by their point size (how a .fon indexes them) but named by
their pixel cell, since the cell is the font-size you actually use: the 8pt and
10pt strikes below come out as `sans-13` and `sans-16`.

    ./scripts/fon2woff2.py packages/playground/sserife.fon --list
    ./scripts/fon2woff2.py packages/playground/sserife.fon --sizes 8 10 \
        --outdir packages/playground

Requires: fonttools, brotli.
"""

# here be slop
# no way was i gonna write this
# o7 claude

import argparse
import os
import struct

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import newTable

PPU = 100  # font units per pixel
VERSION = "1.000"
RT_FONT = 0x8008

# In an OEM font, 0x00-0x1F and 0x7F carry graphics, not control codes. Python's
# cp437 codec decodes them as C0 controls, so those slots are mapped by hand.
CP437_GRAPHICS = {
    0x01: 0x263A, 0x02: 0x263B, 0x03: 0x2665, 0x04: 0x2666, 0x05: 0x2663,
    0x06: 0x2660, 0x07: 0x2022, 0x08: 0x25D8, 0x09: 0x25CB, 0x0A: 0x25D9,
    0x0B: 0x2642, 0x0C: 0x2640, 0x0D: 0x266A, 0x0E: 0x266B, 0x0F: 0x263C,
    0x10: 0x25BA, 0x11: 0x25C4, 0x12: 0x2195, 0x13: 0x203C, 0x14: 0x00B6,
    0x15: 0x00A7, 0x16: 0x25AC, 0x17: 0x21A8, 0x18: 0x2191, 0x19: 0x2193,
    0x1A: 0x2192, 0x1B: 0x2190, 0x1C: 0x221F, 0x1D: 0x2194, 0x1E: 0x25B2,
    0x1F: 0x25BC, 0x7F: 0x2302,
}


def code_to_unicode(charset):
    """Map font code points to Unicode. charset: 'ansi' (cp1252) or 'oem' (cp437)."""
    codec = "cp1252" if charset == "ansi" else "cp437"
    out = {}
    for code in range(1, 256):  # 0 would map to U+0000; nothing to encode there
        if charset == "oem" and code in CP437_GRAPHICS:
            out[code] = CP437_GRAPHICS[code]
            continue
        try:
            uni = ord(bytes([code]).decode(codec))
        except UnicodeDecodeError:
            continue  # unassigned cp1252 slot
        if uni < 0x20:
            continue  # a control code is not worth a cmap entry
        out[code] = uni
    return out


# --- .fon / .fnt parsing ----------------------------------------------------


def ne_font_resources(data):
    """Yield (offset, length) for each RT_FONT resource in an NE executable."""
    if data[:2] != b"MZ":
        raise ValueError("not an MZ executable")
    (ne_off,) = struct.unpack_from("<H", data, 0x3C)
    if data[ne_off : ne_off + 2] != b"NE":
        raise ValueError("not an NE executable")
    (rtab,) = struct.unpack_from("<H", data, ne_off + 0x24)
    rtab += ne_off
    (shift,) = struct.unpack_from("<H", data, rtab)
    p = rtab + 2
    out = []
    while True:
        (type_id,) = struct.unpack_from("<H", data, p)
        if type_id == 0:
            return out
        (count,) = struct.unpack_from("<H", data, p + 2)
        p += 8
        for _ in range(count):
            off, length = struct.unpack_from("<HH", data, p)
            if type_id == RT_FONT:
                out.append((off << shift, length << shift))
            p += 12


def parse_fnt(b):
    """Parse a Windows FNT (v2/v3) bitmap font resource header."""
    f = {}
    (f["version"],) = struct.unpack_from("<H", b, 0x00)
    (f["size"],) = struct.unpack_from("<I", b, 0x02)
    f["copyright"] = b[0x06:0x42].split(b"\0")[0].decode("latin-1")
    (f["type"],) = struct.unpack_from("<H", b, 0x42)
    f["is_vector"] = bool(f["type"] & 1)  # bit 0: stroke font, not a bitmap
    (f["points"],) = struct.unpack_from("<H", b, 0x44)
    (f["ascent"],) = struct.unpack_from("<H", b, 0x4A)
    (f["internal_leading"],) = struct.unpack_from("<H", b, 0x4C)
    (f["external_leading"],) = struct.unpack_from("<H", b, 0x4E)
    f["italic"] = b[0x50]
    (f["weight"],) = struct.unpack_from("<H", b, 0x53)
    f["charset"] = b[0x55]
    (f["pix_height"],) = struct.unpack_from("<H", b, 0x58)
    (f["avg_width"],) = struct.unpack_from("<H", b, 0x5B)
    (f["max_width"],) = struct.unpack_from("<H", b, 0x5D)
    f["first_char"] = b[0x5F]
    f["last_char"] = b[0x60]
    f["default_char"] = b[0x61]
    f["break_char"] = b[0x62]
    (f["face"],) = struct.unpack_from("<I", b, 0x69)
    f["face_name"] = b[f["face"] :].split(b"\0")[0].decode("latin-1") if f["face"] else ""

    # Glyph table: v2 entries are (width:u16, offset:u16), v3 (width:u16, offset:u32).
    if f["version"] == 0x0200:
        entry_size, fmt, table = 4, "<HH", 0x76
    else:
        entry_size, fmt, table = 6, "<HI", 0x94
    n = f["last_char"] - f["first_char"] + 2  # +1 for the absolute-space sentinel
    f["glyph_table"] = [
        struct.unpack_from(fmt, b, table + i * entry_size) for i in range(n)
    ]
    return f


def glyph_bitmap(b, f, index):
    """Return (width, rows) for a glyph; rows are lists of 0/1, top row first."""
    width, offset = f["glyph_table"][index]
    height = f["pix_height"]
    if width == 0:
        return width, []
    rows = [[0] * width for _ in range(height)]
    # Stored column-major: ceil(w/8) strips of `height` bytes, one bit per pixel.
    for col in range((width + 7) // 8):
        base = offset + col * height
        for y in range(height):
            byte = b[base + y]
            for bit in range(8):
                x = col * 8 + bit
                if x < width:
                    rows[y][x] = (byte >> (7 - bit)) & 1
    return width, rows


# --- pixels -> contours -----------------------------------------------------


def _turn_rank(din, dout):
    """0 = right turn, 1 = straight, 2 = left turn, 3 = reversal."""
    cross = din[0] * dout[1] - din[1] * dout[0]
    dot = din[0] * dout[0] + din[1] * dout[1]
    if cross < 0:
        return 0
    if cross == 0:
        return 1 if dot > 0 else 3
    return 2


def _simplify(points):
    """Drop points in the middle of a straight run, so a pixel run is one edge."""
    out = []
    for i, (cx, cy) in enumerate(points):
        px, py = points[i - 1]
        nx, ny = points[(i + 1) % len(points)]
        if (cx - px) * (ny - cy) - (cy - py) * (nx - cx) != 0:
            out.append((cx, cy))
    return out


def trace_contours(pixels):
    """Trace unit cells (x, y, y up) into closed contours, filled area on the left.

    Outer contours come out counter-clockwise and holes clockwise.
    """
    pixels = set(pixels)
    edges = {}

    def add(a, b):
        edges.setdefault(a, []).append(b)

    for x, y in pixels:
        if (x, y - 1) not in pixels:
            add((x, y), (x + 1, y))  # bottom, travel +x
        if (x + 1, y) not in pixels:
            add((x + 1, y), (x + 1, y + 1))  # right, travel +y
        if (x, y + 1) not in pixels:
            add((x + 1, y + 1), (x, y + 1))  # top, travel -x
        if (x - 1, y) not in pixels:
            add((x, y + 1), (x, y))  # left, travel -y

    contours = []
    for start in list(edges):
        while edges.get(start):
            contour = [start]
            cur = start
            nxt = edges[cur].pop(0)
            din = (nxt[0] - cur[0], nxt[1] - cur[1])
            cur = nxt
            while cur != start:
                contour.append(cur)
                options = edges.get(cur)
                if not options:
                    raise ValueError(f"open contour at {cur}")
                # Where two diagonal cells pinch, two edges leave one point;
                # take the sharpest right turn so the lobes stay separate.
                i = min(
                    range(len(options)),
                    key=lambda k: _turn_rank(
                        din, (options[k][0] - cur[0], options[k][1] - cur[1])
                    ),
                )
                nxt = options.pop(i)
                din = (nxt[0] - cur[0], nxt[1] - cur[1])
                cur = nxt
            simplified = _simplify(contour)
            if len(simplified) >= 3:
                contours.append(simplified)
    return contours


# --- font building ----------------------------------------------------------


def glyph_pixels(b, f, code):
    """Filled cells for a character, y up with y=0 on the baseline."""
    w, rows = glyph_bitmap(b, f, code - f["first_char"])
    asc = f["ascent"]
    return w, {
        (x, asc - 1 - y) for y, row in enumerate(rows) for x, p in enumerate(row) if p
    }


def build_glyph(pixels):
    """Return (glyph, lsb); lsb must equal the outline's xMin per the spec."""
    pen = TTGlyphPen(None)
    for contour in trace_contours(pixels):
        # Tracing gives outer contours counter-clockwise; TrueType wants them
        # clockwise, so reverse. Holes flip too and stay opposite.
        contour = list(reversed(contour))
        pen.moveTo((contour[0][0] * PPU, contour[0][1] * PPU))
        for x, y in contour[1:]:
            pen.lineTo((x * PPU, y * PPU))
        pen.closePath()
    return pen.glyph(), min((x for x, _ in pixels), default=0) * PPU


def ink_height(b, f, ch):
    """Height of a character's ink above the baseline, in pixels (0 if absent)."""
    if not f["first_char"] <= ord(ch) <= f["last_char"]:
        return 0
    _, pixels = glyph_pixels(b, f, ord(ch))
    return (max(y for _, y in pixels) + 1) if pixels else 0


def build_strike(raw, f, src_name, family, charset, out_woff2, out_ttf=None):
    height, asc = f["pix_height"], f["ascent"]
    upem = height * PPU
    unicode_of = code_to_unicode(charset)

    # Some fonts fill unused code points with a copy of dfDefaultChar's
    # placeholder; drop those so text falls back instead of drawing a bogus
    # glyph. Only when the default glyph has ink -- where dfDefaultChar is
    # blank (Terminal points it at space) this would delete every blank glyph.
    placeholder = glyph_bitmap(raw, f, f["default_char"])
    if not any(any(row) for row in placeholder[1]):
        placeholder = None

    glyph_order = [".notdef"]
    glyphs, metrics, cmap = {}, {}, {}

    nd_w, nd_pixels = glyph_pixels(raw, f, f["first_char"] + f["default_char"])
    glyphs[".notdef"], nd_lsb = build_glyph(nd_pixels)
    metrics[".notdef"] = (nd_w * PPU, nd_lsb)

    for code in range(f["first_char"], f["last_char"] + 1):
        if placeholder and glyph_bitmap(raw, f, code - f["first_char"]) == placeholder:
            continue
        uni = unicode_of.get(code)
        if uni is None:
            continue
        name = f"uni{uni:04X}"
        w, pixels = glyph_pixels(raw, f, code)
        glyph_order.append(name)
        glyphs[name], lsb = build_glyph(pixels)
        metrics[name] = (w * PPU, lsb)
        cmap[uni] = name

    fb = FontBuilder(upem, isTTF=True)
    fb.setupGlyphOrder(glyph_order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(
        ascent=asc * PPU,
        descent=-(height - asc) * PPU,
        lineGap=f["external_leading"] * PPU,
    )
    fb.setupNameTable(
        {
            # Provenance is stated factually; no third-party marks are used as
            # identifying names. See --help for how to change any of this.
            "copyright": (
                "Outlines mechanically traced from a bitmap strike; "
                "typeface designs are not subject to copyright in the US."
            ),
            "familyName": family,
            "styleName": "Regular",
            "uniqueFontIdentifier": f"{family}; {VERSION}",
            "fullName": family,
            "version": f"Version {VERSION}",
            "psName": f"{family}-Regular",
            "description": (
                f"Traced from the {f['points']}pt strike ({height}px cell) of "
                f"{src_name}. Unhinted; one em equals one {height}px cell, so "
                f"font-size: {height}px renders at native 1:1 scale."
            ),
        }
    )
    fb.setupOS2(
        version=4,
        sTypoAscender=asc * PPU,
        sTypoDescender=-(height - asc) * PPU,
        sTypoLineGap=f["external_leading"] * PPU,
        usWinAscent=asc * PPU,
        usWinDescent=(height - asc) * PPU,
        sxHeight=ink_height(raw, f, "x") * PPU,
        sCapHeight=ink_height(raw, f, "H") * PPU,
        usWeightClass=f["weight"],
        usWidthClass=5,
        fsType=0,
        fsSelection=(1 << 6) | (1 << 7),  # REGULAR | USE_TYPO_METRICS
        achVendID="NONE",
        panose=dict(  # text sans-serif, normal weight, modern proportion
            bFamilyType=2,
            bSerifStyle=11,
            bWeight=5,
            bProportion=3,
            bContrast=0,
            bStrokeVariation=0,
            bArmStyle=0,
            bLetterForm=0,
            bMidline=0,
            bXHeight=0,
        ),
        usDefaultChar=0,
        usBreakChar=32,
    )
    fb.setupPost(keepGlyphNames=False)  # post 3.0: smaller, names unused on the web

    # --- no hinting ---
    # Nothing above emits fpgm/prep/cvt or any glyph instructions. gasp asks for
    # grayscale only, with grid-fitting off.
    gasp = newTable("gasp")
    gasp.version = 1
    gasp.gaspRange = {0xFFFF: 0x000A}  # DOGRAY | SYMMETRIC_SMOOTHING, no GRIDFIT
    fb.font["gasp"] = gasp

    head = fb.font["head"]
    # baseline at y=0 | lsb at x=0 | force integer ppem (keeps pixels on grid).
    # Bits 2 and 4 (instructions affect size / advance) stay clear: no hinting.
    head.flags = 0b1011
    head.lowestRecPPEM = height
    fb.font["maxp"].maxZones = 1

    if out_ttf:
        fb.save(out_ttf)
    fb.font.flavor = "woff2"
    fb.save(out_woff2)
    return len(glyph_order)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("src", help="input .fon file")
    ap.add_argument("--sizes", type=int, nargs="+", default=[8, 10],
                    help="point sizes of the strikes to extract")
    ap.add_argument("--outdir", default=".")
    ap.add_argument("--prefix", default="sans",
                    help="family/file name stem; the pixel cell height is "
                         "appended, e.g. 'sans' -> family and file 'sans-13'")
    ap.add_argument("--ttf", action="store_true", help="also write .ttf alongside")
    ap.add_argument("--charset", choices=("auto", "ansi", "oem"), default="auto",
                    help="source encoding; 'auto' reads dfCharSet (0=ansi/cp1252, "
                         "255=oem/cp437)")
    ap.add_argument("--list", action="store_true", help="list strikes and exit")
    args = ap.parse_args()

    data = open(args.src, "rb").read()
    src_name = os.path.basename(args.src)
    strikes = {}
    for off, ln in ne_font_resources(data):
        raw = data[off : off + ln]
        f = parse_fnt(raw)
        strikes[f["points"]] = (raw, f)

    if args.list:
        for pt, (_, f) in sorted(strikes.items()):
            cs = "oem/cp437" if f["charset"] == 255 else (
                "ansi/cp1252" if f["charset"] == 0 else f"charset {f['charset']}")
            print(
                f"{pt:2d}pt  cell {f['pix_height']:2d}px  ascent {f['ascent']:2d}  "
                f"chars {f['first_char']}-{f['last_char']}  {cs}"
                f"{'  VECTOR (unsupported)' if f['is_vector'] else ''}  "
                f"{f['face_name']}"
            )
        return

    for pt in args.sizes:
        if pt not in strikes:
            raise SystemExit(
                f"no {pt}pt strike in {src_name}; have {sorted(strikes)}"
            )
        raw, f = strikes[pt]
        if f["is_vector"]:
            raise SystemExit(
                f"{src_name} {pt}pt is a vector (stroke) font, not a bitmap "
                f"strike; this converter only handles raster FNTs"
            )
        charset = args.charset
        if charset == "auto":
            charset = "oem" if f["charset"] == 255 else "ansi"
        # Named by pixel cell, not points: the cell is the font-size to use.
        family = f"{args.prefix}-{f['pix_height']}"
        woff2 = os.path.join(args.outdir, f"{family}.woff2")
        ttf = os.path.join(args.outdir, f"{family}.ttf") if args.ttf else None
        n = build_strike(raw, f, src_name, family, charset, woff2, ttf)
        print(
            f"{pt}pt strike -> {family}: {n} glyphs, {f['pix_height']}px cell, "
            f"{charset}, upem {f['pix_height'] * PPU}, "
            f"{os.path.getsize(woff2)} bytes -> {woff2}"
        )


if __name__ == "__main__":
    main()
