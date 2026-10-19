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
    case "mouse-button-event":
      handleMouseButtonEvent(data);
      break;
    case "mouse-motion-event":
      handleMouseMotionEvent(data);
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

  let attach;
  let wasWindowOpen: boolean | undefined;
  let updateWindow = async (tk: Tk) => {
    const windowOpen = tk.hasWindow();

    if (windowOpen && offscreenCanvas) {
      await (attach ??= tk.attachCanvas(offscreenCanvas));
    }

    if (wasWindowOpen === undefined || windowOpen !== wasWindowOpen) {
      wasWindowOpen = windowOpen;
      self.postMessage(workerMessage({ type: "window-change", open: windowOpen }));
    }

    tk.redraw(1);
  };

  let lastDrawTime: number | undefined;
  let lastDraw: Promise<void> | undefined;
  const drawFrame = async (time: number) => {
    // the redraw loop and yield can both run in one animation frame; draw once
    if (!tk) {
      return;
    }
    if (time !== lastDrawTime) {
      lastDrawTime = time;
      lastDraw = undefined;
    }
    await (lastDraw ??= updateWindow(tk));
  };

  try {
    tcl = createTcl({
      handleStdout(value) {
        stdoutBuf += value;
        if (performance.now() - stdoutLastFlush >= 10) {
          flushStdout();
        }
      },
      yield(resume) {
        requestAnimationFrame((time) => {
          drawFrame(time)
            .catch((error) => {
              reportError(error);
            })
            .finally(resume);
        });
      },
    });

    tk = Tk.create();
    tk.install(tcl);

    requestAnimationFrame(function redraw(time) {
      if (!tk) {
        return;
      }

      drawFrame(time)
        .then(() => {
          requestAnimationFrame(redraw);
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
    await updateWindow(tk);
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
    self.close();
  }
}

function handleKeyEvent({ event }: Extract<HostMessage, { type: "key-event" }>) {
  if (!tk) {
    return;
  }
  switch (event.eventType) {
    case "down":
      tk.handleKeyDown(".", event.key, event.location, event.modifiers);
      break;
    case "up":
      tk.handleKeyUp(".", event.key, event.location, event.modifiers);
      break;
    default:
      impossible(event.eventType);
  }
}

function handleMouseButtonEvent({ event }: Extract<HostMessage, { type: "mouse-button-event" }>) {
  if (!tk) {
    return;
  }
  switch (event.eventType) {
    case "down":
      tk.handleMouseDown(".", event.button, event.modifiers);
      break;
    case "up":
      tk.handleMouseUp(".", event.button, event.modifiers);
      break;
    default:
      impossible(event.eventType);
  }
}

function handleMouseMotionEvent({ event }: Extract<HostMessage, { type: "mouse-motion-event" }>) {
  if (!tk) {
    return;
  }
  tk.handleMouseMove(".", event.x, event.y);
}

self.postMessage(workerMessage({ type: "ready" }));
