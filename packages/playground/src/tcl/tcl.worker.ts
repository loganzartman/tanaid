import { createTcl, Tcl, Tk } from "tanaid-tcl";
import { workerMessage, type HostMessage } from "./messages";
import { impossible } from "../impossible";

let tk: Tk | undefined;
let offscreenCanvas: OffscreenCanvas | undefined;

self.onmessage = async ({ data }: { data: HostMessage }) => {
  switch (data.type) {
    case "init":
      await handleInit(data);
      break;
    case "run":
      await handleRun(data);
      break;
    case "key-event":
      handleKeyEvent(data);
      break;
    default:
      impossible(data);
  }
};

async function handleInit(data: Extract<HostMessage, { type: "init" }>) {
  offscreenCanvas = data.offscreenCanvas;
  self.postMessage(workerMessage({ type: "init" }));
}

async function handleRun({ source }: Extract<HostMessage, { type: "run" }>) {
  if (!offscreenCanvas) {
    throw new Error("not initialized");
  }

  let tcl: Tcl | undefined;
  let stopped = false;

  let stdoutBuf = "";
  let stdoutLastFlush = performance.now();
  const flushStdout = () => {
    if (stdoutBuf.length > 0) {
      self.postMessage(workerMessage({ type: "stdout", value: stdoutBuf }));
      stdoutBuf = "";
    }
    stdoutLastFlush = performance.now();
  };

  try {
    tcl = createTcl({
      handleStdout(value) {
        stdoutBuf += value;
        if (performance.now() - stdoutLastFlush >= 10) {
          flushStdout();
        }
      },
    });

    tk = Tk.create();
    tk.install(tcl);

    let wasWindowOpen = false;
    let didAttach = false;

    requestAnimationFrame(async function redraw() {
      if (stopped) {
        return;
      }

      try {
        if (tk) {
          const windowOpen = tk.hasWindow();

          if (windowOpen && offscreenCanvas && !didAttach) {
            didAttach = true;
            await tk.attachCanvas(offscreenCanvas);
          }

          if (windowOpen !== wasWindowOpen) {
            wasWindowOpen = windowOpen;
            self.postMessage(workerMessage({ type: "window-change", open: windowOpen }));
          }

          tk.redraw(1);
        }
      } catch (error) {
        reportError(error);
        return;
      }

      requestAnimationFrame(redraw);
    });

    let lastCountPending: number | undefined;
    const handleEventLoopStatus = (countPending: number) => {
      if (countPending !== lastCountPending) {
        self.postMessage(workerMessage({ type: "event-loop-status", status: { countPending } }));
        lastCountPending = countPending;
      }
      flushStdout();
    };

    const value = await tcl.run(source, { handleEventLoopStatus });
    await tcl.run("tkwait window .", { handleEventLoopStatus });

    // TODO: not reached if sitting in tkwait; split into result + done
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
    flushStdout();
    stopped = true;
    tcl?.free();
    tk?.free();
  }
}

function handleKeyEvent({ event, eventType }: Extract<HostMessage, { type: "key-event" }>) {
  if (!tk) {
    return;
  }
  switch (eventType) {
    case "down":
      tk.handleKeyDown(".", event.key, event.location);
      break;
    case "up":
      tk.handleKeyUp(".", event.key, event.location);
      break;
    default:
      impossible(eventType);
  }
}

self.postMessage(workerMessage({ type: "ready" }));
