import TclWorker from "./tcl.worker.ts?worker";
import { hostMessage, type EventLoopStatus, type WorkerMessage } from "./messages";
import { impossible } from "../impossible";

export type TclOutcome =
  | { status: "running" }
  | { status: "ok"; value: string }
  | { status: "error"; message: string };

export type State = "idle" | "running";

export type RunOptions = { source: string };

export type TclRunner = {
  getState: () => State;
  run: (options: RunOptions) => Promise<string>;
  stop: () => void;
};

/**
 * A handle that manages a worker to run Tcl code.
 */
export function createTclRunner({
  onStdout,
  onEventLoopStatus,
  onStateChanged,
  onWindowChanged,
}: {
  /** Receives buffered text from stdout. */
  onStdout?: (value: string) => void;
  onEventLoopStatus?: (status: EventLoopStatus) => void;
  /** The runner has changed state. */
  onStateChanged?: (state: State) => void;
  /** The Tk window has been updated */
  onWindowChanged?: (status: { open: boolean; canvas: HTMLCanvasElement }) => void;
}): TclRunner {
  let worker: Worker | undefined;

  const { getState, setState } = (() => {
    let state: State = "idle";
    return {
      getState: () => state,
      setState: (newState: State) => {
        state = newState;
        onStateChanged?.(newState);
      },
    };
  })();

  const replaceWorker = async () => {
    if (worker !== undefined) {
      worker.terminate();
    }
    worker = new TclWorker();

    const ready = Promise.withResolvers<void>();
    const init = Promise.withResolvers<void>();
    const result = Promise.withResolvers<string | Error>();

    const canvas = document.createElement("canvas");
    const offscreenCanvas = canvas.transferControlToOffscreen();

    worker.onmessage = ({ data }: { data: WorkerMessage }) => {
      switch (data.type) {
        case "ready":
          ready.resolve();
          break;
        case "result":
          result.resolve(data.result);
          break;
        case "init":
          init.resolve();
          break;
        case "window-change":
          onWindowChanged?.({
            open: data.open,
            canvas,
          });
          break;
        case "stdout":
          onStdout?.(data.value);
          break;
        case "event-loop-status":
          onEventLoopStatus?.(data.status);
          break;
        default:
          impossible(data);
      }
    };

    await ready.promise;

    worker.postMessage(hostMessage({ type: "init", offscreenCanvas }), [offscreenCanvas]);

    await init.promise;

    return { worker, resultPromise: result.promise };
  };

  const run = async ({ source }: RunOptions) => {
    setState("running");
    const { worker, resultPromise } = await replaceWorker();
    worker.postMessage(hostMessage({ type: "run", source }));
    const result = await resultPromise;
    setState("idle");

    if (result instanceof Error) {
      throw result;
    }
    return result;
  };

  const stop = () => {
    worker?.terminate();
    setState("idle");
  };

  return { run, stop, getState };
}

export class InterruptedError extends Error {}
