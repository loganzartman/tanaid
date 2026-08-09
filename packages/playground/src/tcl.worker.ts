import { Interpreter } from "tanaid";

self.onmessage = ({ data: { source } }) => {
  try {
    const interp = Interpreter.create({
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
  }
};

self.postMessage({ type: "ready" });
