/**
 * `<pixel-perfect>` renders its children on the device pixel grid, so bitmap
 * fonts and 1px rules land on exact device pixels at any `devicePixelRatio`.
 *
 * Children are authored in *design pixels*: `font-size: 16px` for a font whose
 * cell is 16px, `padding: 8px` for eight of those pixels. The element applies
 * two scales, because two different stages quantize:
 *
 *   --us  An integer, applied with `zoom`, so it lands before layout. Every
 *         authored `1px` becomes `us` layout px, which keeps glyph advances on
 *         whole device pixels — HarfBuzz rounds them to 1/64 CSS px and Gecko
 *         to whole device px, so anything fractional here drifts along a line.
 *         `zoom` rather than `calc()` so that CSS this element does not own
 *         (a UA sheet, a library's injected styles) scales too.
 *   --gs  The leftover fraction, applied with `transform`. Paint-time only,
 *         and nothing quantizes after it, so it is the safe place for a
 *         non-integer factor. It brings the result back to `ceil(dpr)` device
 *         pixels per design pixel.
 *
 * The element owns its own box and is not meant to be styled; put padding,
 * `display`, and the rest on a child. There is no shadow root either, so the
 * children keep the ancestry the author gave them: code that reconciles
 * `getBoundingClientRect` against `offsetWidth` or walks `parentNode` —
 * CodeMirror does both — reads a slotted subtree inconsistently.
 *
 * Assumes fonts whose advances are a whole number of font pixels, and that the
 * element itself sits at an integer device pixel offset.
 */

const MAX_UNIT_SCALE = 16;
const EPSILON = 1e-6;

/** Smallest integer scale that puts one design pixel on a whole device pixel. */
function unitScaleFor(dpr: number) {
  for (let unitScale = 1; unitScale <= MAX_UNIT_SCALE; ++unitScale) {
    if (Math.abs(unitScale * dpr - Math.round(unitScale * dpr)) < EPSILON) {
      return unitScale;
    }
  }
  return 1;
}

export class PixelPerfect extends HTMLElement {
  #resolution: MediaQueryList | null = null;
  #onResolutionChange = () => this.refresh();

  connectedCallback() {
    // The transform is paint-only, so the layout box has to be pre-divided for
    // the scaled result to fill the parent.
    this.style.cssText = `
      display: block;
      width: calc(100% / var(--gs));
      transform-origin: 0 0;
      transform: scale(var(--gs));
      zoom: var(--us);
    `;
    this.refresh();
  }

  disconnectedCallback() {
    this.#resolution?.removeEventListener("change", this.#onResolutionChange);
    this.#resolution = null;
  }

  refresh() {
    const dpr = window.devicePixelRatio;
    // Without `zoom` there is no way to scale layout by an integer, so fall
    // back to design pixels at 1:1 and let the transform do everything.
    const unitScale = CSS.supports("zoom", "2") ? unitScaleFor(dpr) : 1;

    this.style.setProperty("--us", String(unitScale));
    this.style.setProperty("--gs", String(Math.ceil(dpr) / (dpr * unitScale)));

    this.#watchResolution(dpr);
  }

  #watchResolution(dpr: number) {
    this.#resolution?.removeEventListener("change", this.#onResolutionChange);
    // Fires when the ratio changes: browser zoom, or a move to another display.
    this.#resolution = window.matchMedia(`(resolution: ${dpr}dppx)`);
    this.#resolution.addEventListener("change", this.#onResolutionChange, { once: true });
  }
}

customElements.define("pixel-perfect", PixelPerfect);
