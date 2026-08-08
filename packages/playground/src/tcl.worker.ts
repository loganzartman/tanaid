import { run_tcl } from "tanaid";

self.onmessage = ({ data: { source } }) => {
  try {
    self.postMessage({
      type: "result",
      value: run_tcl(source),
    });
  } catch (error) {
    self.postMessage({
      type: "error",
      error: String(error),
    });
  }
};

self.postMessage({ type: "ready" });
