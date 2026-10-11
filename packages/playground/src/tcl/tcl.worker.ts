import { createTcl, Tcl, Tk } from "tanaid-tcl";
import { workerMessage, type HostMessage } from "./messages";
import { impossible } from "../impossible";

let tcl: Tcl | undefined;
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

  let stdoutBuf = "";
  let stdoutLastFlush = performance.now();
  const flushStdout = () => {
    if (stdoutBuf.length > 0) {
      self.postMessage(workerMessage({ type: "stdout", value: stdoutBuf }));
      stdoutBuf = "";
    }
    stdoutLastFlush = performance.now();
  };

  let didAttach = false;
  let wasWindowOpen: boolean | undefined;
  let af: number | undefined;
  let updateWindow = async () => {
    if (!tk) {
      return;
    }

    const windowOpen = tk.hasWindow();

    if (windowOpen && offscreenCanvas && !didAttach) {
      didAttach = true;
      await tk.attachCanvas(offscreenCanvas);
    }

    if (wasWindowOpen === undefined || windowOpen !== wasWindowOpen) {
      wasWindowOpen = windowOpen;
      self.postMessage(workerMessage({ type: "window-change", open: windowOpen }));
    }

    tk.redraw(1);
  };

  let stopped = false;
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

    af = requestAnimationFrame(function redraw() {
      if (stopped) {
        return;
      }

      updateWindow()
        .then(() => {
          af = requestAnimationFrame(redraw);
        })
        .catch((error) => {
          reportError(error);
        });
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
    await updateWindow();
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
    stopped = true;
    tcl?.free();
    tcl = undefined;
    tk?.free();
    tk = undefined;
    if (af !== undefined) {
      cancelAnimationFrame(af);
    }
    flushStdout();
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
