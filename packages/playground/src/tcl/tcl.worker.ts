import { createTcl, Tcl, Tk } from "tanaid-tcl";
import { workerMessage, type HostMessage } from "./messages";
import { impossible } from "../impossible";

let offscreenCanvas: OffscreenCanvas | undefined;
let abortController: AbortController | undefined;

self.onmessage = async ({ data }: { data: HostMessage }) => {
  switch (data.type) {
    case "init":
      await handleInit(data);
      break;
    case "run":
      await handleRun(data);
      break;
    case "stop":
      await handleStop();
      break;
    default:
      impossible(data);
  }
};

async function handleInit(data: Extract<HostMessage, { type: "init" }>) {
  offscreenCanvas = data.offscreenCanvas;
  self.postMessage(workerMessage({ type: "init" }));
}

async function handleStop() {
  abortController?.abort("stop requested");
}

async function handleRun({ source }: Extract<HostMessage, { type: "run" }>) {
  if (!offscreenCanvas) {
    throw new Error("not initialized");
  }
  abortController = new AbortController();
  let tcl: Tcl | undefined;
  let tk: Tk | undefined;
  let stopped = false;

  try {
    tcl = createTcl({
      handleStdout(value) {
        self.postMessage(workerMessage({ type: "stdout", value }));
      },
    });

    tk = Tk.create();
    tk.install(tcl);
    await tk.attachCanvas(offscreenCanvas);

    requestAnimationFrame(function redraw() {
      if (stopped) {
        return;
      }
      try {
        tk?.redraw(1);
      } catch (error) {
        console.error("Redraw error", error);
      }
      requestAnimationFrame(redraw);
    });

    const value = await tcl.run(source, {
      abortSignal: abortController.signal,
      handleEventLoopStatus(countPending: number) {
        self.postMessage(workerMessage({ type: "event-loop-status", status: { countPending } }));
      },
    });

    self.postMessage(
      workerMessage({
        type: "result",
        result: value,
      }),
    );
  } catch (error) {
    self.postMessage(
      workerMessage({
        type: "result",
        result: error instanceof Error ? error : new Error(String(error)),
      }),
    );
  } finally {
    stopped = true;
    abortController = undefined;
    tcl?.free();
    tk?.free();
  }
}

self.postMessage(workerMessage({ type: "ready" }));
