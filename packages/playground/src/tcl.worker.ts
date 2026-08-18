import { Interpreter } from "tanaid";

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
    let eventLoopEmpty = Promise.withResolvers<void>();
    interp = Interpreter.create({
      handleStdout(value) {
        stdoutBuffer.push(value);
        if (performance.now() - t0 > 16) {
          flushStdout();
        }
      },
      setTimeout(callback, delayMs) {
        return globalThis.setTimeout(callback, delayMs);
      },
      clearTimeout(timeoutId) {
        globalThis.clearTimeout(timeoutId as number);
      },
      handleEventLoopEmpty() {
        eventLoopEmpty.resolve();
      },
    });

    const value = interp.run(source);
    flushStdout();
    self.postMessage({
      type: "result",
      value,
    });

    await eventLoopEmpty.promise;
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
