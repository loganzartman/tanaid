import { Interpreter } from "tanaid";

self.onmessage = ({ data: { source } }) => {
  let interp;

  let t0 = performance.now();
  const stdoutBuffer: string[] = [];
  const flushStdout = () => {
    self.postMessage({ type: "stdout", value: stdoutBuffer.join("") });
    stdoutBuffer.length = 0;
    t0 = performance.now();
  };

  try {
    interp = Interpreter.create({
      handleStdout(value) {
        stdoutBuffer.push(value);
        if (performance.now() - t0 > 16) {
          flushStdout();
        }
      },
    });

    const value = interp.run(source);
    flushStdout();
    self.postMessage({
      type: "result",
      value,
    });
  } catch (error) {
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
