import { createEffect, createMemo, createSignal, type Accessor, type ParentProps } from "solid-js";

/**
 * `<PixelPerfect>` renders its children on the device pixel grid, so bitmap
 * fonts, images, and whole-number CSS lengths land on exact device pixels at
 * any `devicePixelRatio`.
 *
 * Children are authored in *design pixels*. One design pixel is always rendered
 * as a whole number of device pixels. The element scales pixels in two steps:
 *
 * 1. Unit scaling:
 * Applied with `zoom`, which adjusts lengths before layout. One design pixel
 * becomes `u` CSS px. This automatically applies to font sizes, padding, etc.
 *
 * 2. Global scaling:
 * Applied with `transform`, which adjusts sizes when rendering. It scales the
 * result down to `ceil(devicePixelRatio)` device pixels per design pixel.
 * So, each design pixel is at least 1 device pixel, and more on HiDPI displays.
 *
 * Special fonts must be used; their advances must be whole numbers.
 * Using fractional lengths (e.g. `0.5px`) will also break the effect.
 */
export function PixelPerfect(props: ParentProps) {
  const dpr = createDevicePixelRatio();

  const scale = createMemo(() => {
    // the unitScale scales CSS pixels to integer device pixels
    // the goal is to get whole device pixels during layout calculation.
    const unitScale = CSS.supports("zoom", "2") ? unitScaleFor(dpr()) : 1;

    // the global scale:
    // 1. reverses the effect of unit scale once layout has happened on whole device pixels
    // 2. rounds up the DPR and applies it, so things are not too small on high-DPI devices
    const globalScale = Math.ceil(dpr()) / (dpr() * unitScale);

    return { unitScale, globalScale };
  });

  // `transform` does not change the layout box, so divide the box by `gs` to
  // leave the scaled result filling the parent.

  // font-smoothing is required for macOS to avoid subpixel antialiasing.
  return (
    <div
      style={{
        "--us": String(scale().unitScale),
        "--gs": String(scale().globalScale),
        display: "block",
        width: "calc(round(100% / var(--gs), 1px))",
        height: "calc(round(100% / var(--gs), 1px))",
        "transform-origin": "0 0",
        transform: "scale(var(--gs))",
        zoom: "var(--us)",
        "box-sizing": "border-box",
        "image-rendering": "pixelated",
        "-webkit-font-smoothing": "none",
        "-moz-osx-font-smoothing": "grayscale",
      }}
    >
      {props.children}
    </div>
  );
}

/**
 * Track `window.devicePixelRatio`, which changes with browser zoom or when the
 * window moves to another display.
 */
function createDevicePixelRatio(): Accessor<number> {
  const [dpr, setDpr] = createSignal(window.devicePixelRatio);

  createEffect(dpr, (value) => {
    // this query stops matching as soon as the DPR changes
    const resolution = window.matchMedia(`(resolution: ${value}dppx)`);
    const onChange = () => setDpr(window.devicePixelRatio);
    resolution.addEventListener("change", onChange);
    return () => resolution.removeEventListener("change", onChange);
  });

  return dpr;
}

/**
 * Compute smallest scale that maps one CSS pixel to an integer number of device pixels.
 *
 * Tries to find a unit scale `us` such that `us * devicePixelRatio` is an integer.
 * Searches the space of integer-valued `us * devicePixelRatio` ("device scale") and computes `us`,
 * rather than the other way around.
 */
function unitScaleFor(dpr: number) {
  const MAX_UNIT_SCALE = 16;

  // the browser's layout engine rounds CSS lengths to a grid.
  // in Blink/WebKit, the cell size is 1/64 CSS pixel; in Gecko, it's 1/60 CSS pixel.
  // the unit scale needs to fall exactly onto a grid cell;
  // else it may be rounded, and the unit scaling will no longer map exactly to a whole device pixel.
  // (the goal is to avoid rounding after multiplying by the unit scale)
  // this value is the least common multiple of the grid cell sizes for all engines.
  const GRID_LCM = 1 / 4;

  // start searching from ceil(dpr) so unit scaling never scales down; global scaling always scales down
  for (let deviceScale = Math.ceil(dpr); ; ++deviceScale) {
    const unitScale = deviceScale / dpr;
    if (unitScale > MAX_UNIT_SCALE) break;

    const gridSteps = unitScale / GRID_LCM;
    if (floatIsInteger(gridSteps)) {
      // clean up float noise
      const unitScaleRounded = Math.round(gridSteps) * GRID_LCM;
      return unitScaleRounded;
    }
  }

  // some DPRs have no scale on the grid at a usable size; e.g. 67% zoom.
  // fall back to the scale that maps 1 design pixel to 1 whole device pixel,
  // and accept that it is off the grid, so layout rounds it.
  return Math.ceil(dpr) / dpr;
}

function floatIsInteger(x: number): boolean {
  return floatEq(x, Math.round(x));
}

function floatEq(a: number, b: number): boolean {
  return Math.abs(a - b) < 1e-6;
}
