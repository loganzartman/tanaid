import TclWorker from "./tcl.worker.ts?worker";
import { hostMessage, type EventLoopStatus, type WorkerMessage } from "./messages";
import { impossible } from "../impossible";

export type TclOutcome =
  | { status: "running" }
  | { status: "ok"; value: string }
  | { status: "error"; message: string };

export type State = "idle" | "running" | "stopping";

export type TclRunnerRunOptions = { source: string };

export type TclRunner = {
  getState: () => State;
  run: (options: TclRunnerRunOptions) => Promise<string>;
  stopIfRunning: () => Promise<void>;
};

export function createTclRunner({
  offscreenCanvas,
  onStdout,
  onEventLoopStatus,
}: {
  offscreenCanvas: OffscreenCanvas;
  onStdout?: (value: string) => void;
  onEventLoopStatus?: (status: EventLoopStatus) => void;
}): TclRunner {
  const worker = new TclWorker();
  let state: State = "idle";
  const ready = Promise.withResolvers<void>();
  const init = Promise.withResolvers<void>();
  let stopped = Promise.withResolvers<void>();
  let result: PromiseWithResolvers<string> | undefined;
  let queuedRun: TclRunnerRunOptions | undefined;
  let queuedResult: PromiseWithResolvers<string> | undefined;

  let didInit = false;
  const initOnce = async () => {
    if (didInit) {
      await init.promise;
      return;
    }
    didInit = true;

    await ready.promise;
    worker.postMessage(
      hostMessage({
        type: "init",
        offscreenCanvas,
      }),
      offscreenCanvas ? [offscreenCanvas] : [],
    );
    await init.promise;
  };

  const stopIfRunning = async () => {
    await initOnce();
    switch (state) {
      case "idle": {
        break;
      }
      case "running": {
        stopped = Promise.withResolvers();
        state = "stopping";
        if (result) {
          result.reject(new InterruptedError());
          result = undefined;
        }
        worker.postMessage(hostMessage({ type: "stop" }));
        await stopped.promise;
        break;
      }
      case "stopping": {
        await stopped.promise;
        break;
      }
      default:
        impossible(state);
    }
  };

  const runNow = async (options: TclRunnerRunOptions) => {
    await initOnce();
    if (state !== "idle") {
      throw new Error("internal error: should be idle");
    }

    state = "running";
    worker.postMessage(hostMessage({ type: "run", source: options.source }));
  };

  const queueRun = (options: TclRunnerRunOptions) => {
    if (queuedResult) {
      queuedResult.promise.catch(() => {});
      queuedResult.reject(new InterruptedError());
    }
    queuedResult = Promise.withResolvers();
    queuedRun = options;
    return queuedResult.promise;
  };

  const runQueued = async () => {
    if (state !== "idle") {
      throw new Error("internal error: should be idle");
    }
    if (!queuedRun || !queuedResult) {
      return Promise.resolve();
    }

    const options = queuedRun;
    result = queuedResult;
    queuedRun = undefined;
    queuedResult = undefined;

    await runNow(options);
    return result;
  };

  const run: TclRunner["run"] = async (options) => {
    switch (state) {
      case "idle": {
        result = Promise.withResolvers();
        await runNow(options);
        return result.promise;
      }
      case "running": {
        let result = queueRun(options);
        await stopIfRunning();
        return result;
      }
      case "stopping": {
        return queueRun(options);
      }
      default:
        impossible(state);
    }
  };

  worker.addEventListener(
    "message",
    ({ data }: { data: WorkerMessage }) => {
      switch (data.type) {
        case "ready": {
          ready.resolve();
          break;
        }
        case "init": {
          init.resolve();
          break;
        }
        case "event-loop-status": {
          onEventLoopStatus?.(data.status);
          break;
        }
        case "stdout": {
          onStdout?.(data.value);
          break;
        }
        case "result": {
          state = "idle";
          stopped.resolve();

          if (typeof data.result === "string") {
            result?.resolve(data.result);
          } else {
            result?.reject(data.result);
          }

          runQueued().catch((error) => {
            console.error(error);
          });
          break;
        }
        default:
          impossible(data);
      }
    },
    false,
  );

  return {
    getState: () => state,
    run,
    stopIfRunning,
  };
}

export class InterruptedError extends Error {}
