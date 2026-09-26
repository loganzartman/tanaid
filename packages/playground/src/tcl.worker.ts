import { Tcl } from "tanaid-tcl";

self.onmessage = async ({ data: { source } }) => {
  let tcl;

  let t0 = performance.now();
  const stdoutBuffer: string[] = [];
  const flushStdout = () => {
    self.postMessage({ type: "stdout", value: stdoutBuffer.join("") });
    stdoutBuffer.length = 0;
    t0 = performance.now();
  };

  try {
    tcl = Tcl.create({
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
            self.postMessage({ type: "error", error: String(error) });
          }
        }, delayMs);
      },
      clearTimeout(timeoutId) {
        globalThis.clearTimeout(timeoutId as number);
      },
    });

    const value = await tcl.run(source, {
      handleEventLoopStatus(countPending: number) {
        self.postMessage({ type: "event_loop_status", countPending });
      },
    });

    flushStdout();
    self.postMessage({
      type: "result",
      value,
    });

    self.postMessage({ type: "done" });
  } catch (error) {
    console.error(error);
    flushStdout();
    self.postMessage({
      type: "error",
      error: String(error),
    });
  } finally {
    tcl?.free();
  }
};

self.postMessage({ type: "ready" });
