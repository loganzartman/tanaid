import TclWorker from "./tcl.worker.ts?worker";
import { hostMessage, type EventLoopStatus, type WorkerMessage } from "./messages";
import { impossible } from "../impossible";
import { raceAbort } from "../race-abort";

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
  let abort: AbortController | undefined;

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

  const stop = () => {
    worker?.terminate();
    abort?.abort(new InterruptedError("stopped"));
    setState("idle");
  };

  const replaceWorker = async () => {
    if (worker !== undefined) {
      worker.terminate();
      abort?.abort(new InterruptedError("interrupted"));
    }
    worker = new TclWorker();
    abort = new AbortController();

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

    worker.onerror = (event) => {
      setState("idle");
      worker?.terminate();
      abort?.abort(new Error(event.message));
    };

    await raceAbort(ready.promise, abort.signal);

    worker.postMessage(hostMessage({ type: "init", offscreenCanvas }), [offscreenCanvas]);

    await raceAbort(init.promise, abort.signal);

    return { worker, abort, resultPromise: result.promise };
  };

  const run = async ({ source }: RunOptions) => {
    setState("running");
    const { worker, abort, resultPromise } = await replaceWorker();
    worker.postMessage(hostMessage({ type: "run", source }));
    const result = await raceAbort(resultPromise, abort.signal);
    setState("idle");

    if (result instanceof Error) {
      throw result;
    }
    return result;
  };

  return { run, stop, getState };
}

export class InterruptedError extends Error {}
