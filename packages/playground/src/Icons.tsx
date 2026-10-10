/*
 * Icons are inline SVG rather than `<img>`: an SVG image is a separate document
 * that can't see the theme's colors. They fill with `--text-color` and not
 * `currentColor`, because 98.css makes a button's `color` transparent.
 */

export function StartIcon() {
  return (
    <svg class="fill-(--text-color)" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M3 3h1v1h2v1h2v1h2v1h2v1h-2v1h-2v1h-2v1h-2v1h-1z" />
    </svg>
  );
}

export function StopIcon() {
  return (
    <svg
      class="fill-(--text-color)"
      width="16"
      height="16"
      viewBox="0 0 16 16"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <path d="M4 4h8v8h-8z" />
    </svg>
  );
}
