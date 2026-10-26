import { createEffect, createMemo, createSignal, Show, type ParentProps, type Ref } from "solid-js";
import type { JSX } from "@solidjs/web";
import { createPixelPerfectScale } from "./PixelPerfect";

export type WindowProps = ParentProps & {
  draggable?: boolean;
  onClose?: () => void;
  onFocusOut?: (event: FocusEvent) => void;
  onPointerDown?: (event: PointerEvent, data: { x: number; y: number }) => void;
  onPointerUp?: (event: PointerEvent, data: { x: number; y: number }) => void;
  onPointerMove?: (event: PointerEvent, data: { x: number; y: number }) => void;
  onPointerCancel?: (event: PointerEvent, data: { x: number; y: number }) => void;
  onKeyDown?: (event: KeyboardEvent) => void;
  onKeyUp?: (event: KeyboardEvent) => void;
  title?: string;
  menuBar?: JSX.Element;
  open?: boolean;
  class?: string;
  ref?: Ref<HTMLDivElement>;
};

export function Window(props: WindowProps) {
  const scale = createPixelPerfectScale();
  const mouseScale = createMemo(() => scale().globalScale * scale().unitScale);
  const [x, setX] = createSignal(() => Math.round(window.innerWidth / 2 / mouseScale()));
  const [y, setY] = createSignal(() => Math.round(window.innerHeight / 2 / mouseScale()));

  let pdown = false;
  let px0 = 0;
  let py0 = 0;
  let wx0 = 0;
  let wy0 = 0;

  let windowRef!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let titlebarRef!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let childrenContainerRef!: HTMLDivElement;

  const handleTitlebarDown = (event: PointerEvent) => {
    windowRef.focus();
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

  const dataFromPointerEvent = (event: PointerEvent) => {
    const rect = childrenContainerRef.getBoundingClientRect();
    return {
      x: (event.clientX - rect.left) / mouseScale(),
      y: (event.clientY - rect.top) / mouseScale(),
    };
  };

  createEffect(
    () => props.onPointerMove,
    (onPointerMove) => {
      const handlePointerMove = (event: PointerEvent) => {
        onPointerMove?.(event, dataFromPointerEvent(event));
      };
      document.addEventListener("pointermove", handlePointerMove, false);
      return () => {
        document.removeEventListener("pointermove", handlePointerMove);
      };
    },
  );

  return (
    <div
      ref={[
        props.ref,
        (ref) => {
          windowRef = ref;
        },
      ]}
      class={`window m-0 flex flex-col ${props.class ?? ""}`}
      tabindex={0}
      style={{
        position: props.draggable ? "absolute" : "unset",
        left: props.draggable ? `${x()}px` : "unset",
        top: props.draggable ? `${y()}px` : "unset",
        transform: props.draggable ? `translate(round(-50%, 1px), round(-50%, 1px))` : "unset",
        display: (props.open ?? true) ? "flex" : "none",
      }}
      onFocusOut={(event) => {
        if (event.target !== event.currentTarget) {
          return;
        }
        props.onFocusOut?.(event);
      }}
      onKeyDown={(event) => {
        if (event.target !== event.currentTarget) {
          return;
        }
        props.onKeyDown?.(event);
      }}
      onKeyUp={(event) => {
        if (event.target !== event.currentTarget) {
          return;
        }
        props.onKeyUp?.(event);
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
      {props.menuBar}
      <div
        ref={childrenContainerRef}
        class="window-body flex flex-1"
        onPointerDown={(event) => props.onPointerDown?.(event, dataFromPointerEvent(event))}
        onPointerUp={(event) => props.onPointerUp?.(event, dataFromPointerEvent(event))}
        onPointerCancel={(event) => props.onPointerCancel?.(event, dataFromPointerEvent(event))}
        onContextMenu={(e) => e.preventDefault()}
      >
        {props.children}
      </div>
    </div>
  );
}
