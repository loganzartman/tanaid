export type Result = string;
export type EventLoopStatus = { countPending: number };

export type HostMessage =
  | { type: "init"; offscreenCanvas: OffscreenCanvas }
  | { type: "stop" }
  | { type: "run"; source: string };
export type WorkerMessage =
  | { type: "ready" }
  | { type: "init" }
  | { type: "stdout"; value: string }
  | { type: "event-loop-status"; status: EventLoopStatus }
  | { type: "result"; result: Result | Error };

export const hostMessage = <const T extends HostMessage>(m: T): T => m;
export const workerMessage = <const T extends WorkerMessage>(m: T): T => m;
