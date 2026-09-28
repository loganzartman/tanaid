import { createEffect, createSignal, type Accessor } from "solid-js";
import TclWorker from "./tcl.worker.ts?worker";

export type TclOutcome =
  | { status: "running" }
  | { status: "ok"; value: string }
  | { status: "error"; message: string };

/**
 * Evaluate `source` in a fresh worker whenever it changes, cancelling the
 * previous evaluation.
 */
export function createTclRunner(source: Accessor<string>, { timeoutMs }: { timeoutMs?: number }) {
  const [outcome, setOutcome] = createSignal<TclOutcome>({ status: "running" });
  const [pendingTimers, setPendingTimers] = createSignal(0);
  const [stdout, setStdout] = createSignal("");

  createEffect(source, (code) => {
    setOutcome({ status: "running" });
    setPendingTimers(0);
    setStdout("");

    // a cancelled evaluation rejects; don't let it overwrite the next one
    let cancelled = false;
    const [cancel, done] = runTcl(code, {
      handleResult(value) {
        if (!cancelled) setOutcome({ status: "ok", value });
      },
      handlePendingTimers(nPending) {
        if (!cancelled) setPendingTimers(nPending);
      },
      handleStdout(value) {
        if (!cancelled) setStdout((prev) => prev + value);
      },
      timeoutMs,
    });
    done.catch((error) => {
      if (!cancelled) setOutcome({ status: "error", message: String(error) });
    });

    return () => {
      cancelled = true;
      cancel();
    };
  });

  return { outcome, pendingTimers, stdout };
}

function runTcl(
  source: string,
  {
    handleResult,
    handlePendingTimers,
    handleStdout,
    timeoutMs,
  }: {
    handleResult: (value: string) => void;
    handlePendingTimers: (nPending: number) => void;
    handleStdout: (value: string) => void;
    timeoutMs?: number;
  },
): [() => void, Promise<void>] {
  const worker = new TclWorker();
  const cancelPromise = Promise.withResolvers<void>();
  const cancel = () => cancelPromise.reject(new Error("cancelled"));
  let timeout: number | null = null;

  return [
    cancel,
    Promise.race([
      new Promise<void>((res, rej) => {
        const readyPromise = Promise.withResolvers<void>();
        readyPromise.promise.then(() => {
          worker.postMessage({ source });
        });

        worker.onmessage = ({ data }) => {
          switch (data.type) {
            case "ready":
              readyPromise.resolve();
              break;
            case "result":
              handleResult(data.value);
              break;
            case "event-loop-status":
              handlePendingTimers(data.countPending);
              break;
            case "done":
              res();
              break;
            case "error":
              rej(String(data.error));
              break;
            case "stdout":
              handleStdout(data.value);
              break;
            default:
              throw new Error(`unknown event type: ${data.type}`);
          }
        };

        worker.onerror = (error) => rej(String(error));
      }),

      ...(timeoutMs !== undefined
        ? [
            new Promise<void>((_, rej) => {
              timeout = window.setTimeout(() => {
                rej(new Error(`timeout: ${timeoutMs}ms`));
              }, timeoutMs);
            }),
          ]
        : []),

      cancelPromise.promise,
    ]).finally(() => {
      worker.terminate();
      if (timeout !== null) {
        clearTimeout(timeout);
      }
    }),
  ];
}
