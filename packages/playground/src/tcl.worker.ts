import { Interpreter } from "tanaid";

self.onmessage = ({ data: { source } }) => {
  const interp = Interpreter.create({
    handleStdout(value) {
      self.postMessage({ type: "stdout", value });
    },
  });

  try {
    self.postMessage({
      type: "result",
      value: interp.run(source),
    });
  } catch (error) {
    self.postMessage({
      type: "error",
      error: String(error),
    });
  } finally {
    interp.free();
  }
};

self.postMessage({ type: "ready" });
