import { Interpreter } from "tanaid-tcl";

self.onmessage = async ({ data: { source } }) => {
  let interp;

  let t0 = performance.now();
  const stdoutBuffer: string[] = [];
  const flushStdout = () => {
    self.postMessage({ type: "stdout", value: stdoutBuffer.join("") });
    stdoutBuffer.length = 0;
    t0 = performance.now();
  };

  try {
    let eventLoop = Promise.withResolvers<void>();

    interp = Interpreter.create({
      handleStdout(value) {
        stdoutBuffer.push(value);
        if (performance.now() - t0 > 16) {
          flushStdout();
        }
      },
      setTimeout(callback, delayMs) {
        return globalThis.setTimeout(() => {
          try {
            callback();
          } catch (error) {
            eventLoop.reject(error);
          }
        }, delayMs);
      },
      clearTimeout(timeoutId) {
        globalThis.clearTimeout(timeoutId as number);
      },
      handleEventLoopStatus(nPending: number) {
        self.postMessage({ type: "pending-timers", value: nPending });
        if (nPending === 0) {
          eventLoop.resolve();
        }
      },
    });

    const value = interp.run(source);
    flushStdout();
    self.postMessage({
      type: "result",
      value,
    });

    await eventLoop.promise;
    flushStdout();

    self.postMessage({ type: "done" });
  } catch (error) {
    console.error(error);
    flushStdout();
    self.postMessage({
      type: "error",
      error: String(error),
    });
  } finally {
    interp?.free();
  }
};

self.postMessage({ type: "ready" });
