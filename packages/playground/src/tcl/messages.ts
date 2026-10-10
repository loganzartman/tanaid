export type Result = string;
export type EventLoopStatus = { countPending: number };

export type SerializedKeyboardEvent = { key: string; location: number };
export type KeyboardEventType = "down" | "up";

export type HostMessage =
  | { type: "init"; offscreenCanvas: OffscreenCanvas }
  | { type: "run"; source: string }
  | { type: "key-event"; eventType: KeyboardEventType; event: SerializedKeyboardEvent };
export type WorkerMessage =
  | { type: "ready" }
  | { type: "init" }
  | { type: "stdout"; value: string }
  | { type: "window-change"; open: boolean }
  | { type: "event-loop-status"; status: EventLoopStatus }
  | { type: "result"; result: Result | Error };

export const hostMessage = <const T extends HostMessage>(m: T): T => m;
export const workerMessage = <const T extends WorkerMessage>(m: T): T => m;
