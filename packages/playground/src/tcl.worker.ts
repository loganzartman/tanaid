import { Interpreter } from "tanaid";

self.onmessage = ({ data: { source } }) => {
  let interp;

  try {
    interp = Interpreter.create({
      handleStdout(value) {
        self.postMessage({ type: "stdout", value });
      },
    });

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
    interp?.free();
  }
};

self.postMessage({ type: "ready" });
