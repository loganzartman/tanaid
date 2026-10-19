import type { TclModifierKeys } from "tanaid-tcl";

export type Result = string;
export type EventLoopStatus = { countPending: number };

export type SerializedKeyboardEvent = {
  eventType: KeyboardEventType;
  key: string;
  location: number;
  modifiers: TclModifierKeys;
};
export type KeyboardEventType = "down" | "up";
export type SerializedMouseButtonEvent = {
  eventType: MouseButtonEventType;
  button: number;
  modifiers: TclModifierKeys;
  x: number;
  y: number;
};
export type MouseButtonEventType = "down" | "up";
export type SerializedMouseMotionEvent = {
  x: number;
  y: number;
};

export type HostMessage =
  | { type: "init"; offscreenCanvas: OffscreenCanvas }
  | { type: "run"; source: string }
  | {
      type: "key-event";
      event: SerializedKeyboardEvent;
    }
  | {
      type: "mouse-button-event";
      event: SerializedMouseButtonEvent;
    }
  | {
      type: "mouse-motion-event";
      event: SerializedMouseMotionEvent;
    };
export type WorkerMessage =
  | { type: "ready" }
  | { type: "init" }
  | { type: "stdout"; value: string }
  | { type: "window-change"; open: boolean }
  | { type: "event-loop-status"; status: EventLoopStatus }
  | { type: "result"; result: Result | Error };

export const hostMessage = <const T extends HostMessage>(m: T): T => m;
export const workerMessage = <const T extends WorkerMessage>(m: T): T => m;
