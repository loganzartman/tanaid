import { createSignal, Show, type ParentProps } from "solid-js";
import { createPixelPerfectScale } from "./PixelPerfect";

export type WindowProps = ParentProps & {
  draggable?: boolean;
  onClose?: () => void;
  title?: string;
  open?: boolean;
};

export function Window(props: WindowProps) {
  const mouseScale = createPixelPerfectScale();
  const [x, setX] = createSignal(() => window.innerWidth / 2 / mouseScale());
  const [y, setY] = createSignal(() => window.innerHeight / 2 / mouseScale());

  let pdown = false;
  let px0 = 0;
  let py0 = 0;
  let wx0 = 0;
  let wy0 = 0;

  // oxlint-disable-next-line no-unassigned-vars
  let titlebarRef!: HTMLDivElement;

  const handleTitlebarDown = (event: PointerEvent) => {
    if (!props.draggable) {
      return;
    }
    if (event.target !== titlebarRef) {
      return;
    }
    event.preventDefault();
    titlebarRef.setPointerCapture(event.pointerId);
    pdown = true;
    px0 = event.clientX;
    py0 = event.clientY;
    wx0 = x();
    wy0 = y();
  };
  const handleTitlebarUp = (_event: PointerEvent) => {
    pdown = false;
  };
  const handleTitlebarMove = (event: PointerEvent) => {
    if (!pdown) {
      return;
    }
    event.preventDefault();
    const px = event.clientX;
    const py = event.clientY;
    setX(Math.round(wx0 + (px - px0) / mouseScale()));
    setY(Math.round(wy0 + (py - py0) / mouseScale()));
  };

  return (
    <div
      class="window absolute m-0"
      style={{
        left: `${x()}px`,
        top: `${y()}px`,
        transform: `translate(-50%, -50%)`,
        display: props.open ? "block" : "none",
      }}
    >
      <div
        ref={titlebarRef}
        class="title-bar"
        onPointerDown={handleTitlebarDown}
        onPointerUp={handleTitlebarUp}
        onPointerMove={handleTitlebarMove}
      >
        <div class="title-bar-text pointer-events-none">{props.title}</div>
        <div class="title-bar-controls">
          <Show when={props.onClose}>
            <button aria-label="Close" onClick={() => props.onClose?.()}></button>
          </Show>
        </div>
      </div>
      <div class="window-body">{props.children}</div>
    </div>
  );
}
