import { EditorState } from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { tcl } from "@sourcebot/codemirror-lang-tcl";
import { emacsTheme } from "./theme.ts";
import TclWorker from "./tcl.worker.ts?worker";
import "./pixel-perfect.ts";

const exampleSelectEl = document.getElementById("example")! as HTMLSelectElement;
const inputContainerEl = document.getElementById("input")! as HTMLElement;
const resultEl = document.getElementById("result")! as HTMLElement;
const pendingTimersEl = document.getElementById("pendingTimers")! as HTMLElement;
const pendingTimersIconNoneEl = document.getElementById("pendingTimersIconNone")! as HTMLElement;
const pendingTimersIconSomeEl = document.getElementById("pendingTimersIconSome")! as HTMLElement;
const stdoutContainerEl = document.getElementById("stdout")! as HTMLElement;
const canvasWindowEl = document.getElementById("canvasWindow")! as HTMLElement;
const canvasWindowTitleBarEl = document.getElementById("canvasWindowTitleBar")! as HTMLElement;
const canvasWindowCloseEl = document.getElementById("canvasWindowClose")! as HTMLButtonElement;
const canvasContainerEl = document.getElementById("canvas")! as HTMLElement;

/** Marks a run that was stopped on purpose, rather than one that failed. */
const CANCELLED = "cancelled";

const stdoutView = new EditorView({
  state: EditorState.create({
    doc: "",
    extensions: [EditorState.readOnly.of(true), drawSelection(), emacsTheme()],
  }),
  parent: stdoutContainerEl,
});

const clearStdout = () => {
  stdoutView.dispatch({
    changes: { from: 0, to: stdoutView.state.doc.length, insert: "" },
  });
};

let stdoutScrollRaf = 0;
const scrollStdoutToEnd = () => {
  stdoutView.scrollDOM.scrollTop = stdoutView.scrollDOM.scrollHeight;
};

const appendStdout = (value: string) => {
  if (!value.length) return;
  stdoutView.dispatch({
    changes: { from: stdoutView.state.doc.length, insert: value },
  });
  if (!stdoutScrollRaf) {
    stdoutScrollRaf = requestAnimationFrame(() => {
      stdoutScrollRaf = 0;
      scrollStdoutToEnd();
    });
  }
};

const showPendingTimers = (nPending: number) => {
  pendingTimersEl.innerText = `${nPending} timers`;
  pendingTimersIconNoneEl.style.display = nPending > 0 ? "none" : "block";
  pendingTimersIconSomeEl.style.display = nPending > 0 ? "block" : "none";
};

const clamp = (x: number, low: number, high: number) => Math.min(Math.max(x, low), high);

/**
 * The canvas window is laid out in design pixels, but pointer events arrive in
 * CSS pixels; `<pixel-perfect>` scales between the two.
 */
const designPixelScale = () =>
  canvasWindowEl.getBoundingClientRect().width / canvasWindowEl.offsetWidth || 1;

let canvasWindowPosition: { left: number; top: number } | null = null;

/** Moves the canvas window, keeping it on screen and on the pixel grid. */
const moveCanvasWindow = (left: number, top: number) => {
  const scale = designPixelScale();
  const maxLeft = window.innerWidth / scale - canvasWindowEl.offsetWidth;
  const maxTop = window.innerHeight / scale - canvasWindowEl.offsetHeight;

  canvasWindowPosition = {
    left: Math.round(clamp(left, 0, Math.max(0, maxLeft))),
    top: Math.round(clamp(top, 0, Math.max(0, maxTop))),
  };
  canvasWindowEl.style.left = `${canvasWindowPosition.left}px`;
  canvasWindowEl.style.top = `${canvasWindowPosition.top}px`;
};

const openCanvasWindow = (canvas: HTMLCanvasElement, width: number, height: number) => {
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  canvasContainerEl.replaceChildren(canvas);
  canvasWindowEl.style.display = "";

  const scale = designPixelScale();
  // a window that hasn't been dragged anywhere yet opens in the middle
  moveCanvasWindow(
    canvasWindowPosition?.left ?? (window.innerWidth / scale - canvasWindowEl.offsetWidth) / 2,
    canvasWindowPosition?.top ?? (window.innerHeight / scale - canvasWindowEl.offsetHeight) / 2,
  );
};

const closeCanvasWindow = () => {
  canvasWindowEl.style.display = "none";
  canvasContainerEl.replaceChildren();
};

canvasWindowTitleBarEl.addEventListener("pointerdown", (event) => {
  if (event.button !== 0 || !canvasWindowPosition) return;
  // capturing the pointer for a drag would swallow the close button's click
  if ((event.target as HTMLElement).closest(".title-bar-controls")) return;

  const scale = designPixelScale();
  const start = { x: event.clientX, y: event.clientY, ...canvasWindowPosition };

  const drag = (moveEvent: PointerEvent) => {
    moveCanvasWindow(
      start.left + (moveEvent.clientX - start.x) / scale,
      start.top + (moveEvent.clientY - start.y) / scale,
    );
  };
  const drop = () => {
    canvasWindowTitleBarEl.removeEventListener("pointermove", drag);
    canvasWindowTitleBarEl.removeEventListener("pointerup", drop);
    canvasWindowTitleBarEl.removeEventListener("pointercancel", drop);
  };

  canvasWindowTitleBarEl.setPointerCapture(event.pointerId);
  canvasWindowTitleBarEl.addEventListener("pointermove", drag);
  canvasWindowTitleBarEl.addEventListener("pointerup", drop);
  canvasWindowTitleBarEl.addEventListener("pointercancel", drop);
  event.preventDefault();
});

// a window dragged to the edge shouldn't end up off screen
window.addEventListener("resize", () => {
  if (canvasWindowPosition && canvasWindowEl.style.display !== "none") {
    moveCanvasWindow(canvasWindowPosition.left, canvasWindowPosition.top);
  }
});

function runTcl(
  source: string,
  {
    handleResult,
    handlePendingTimers,
    handleStdout,
    handleWindow,
    handleNoWindow,
    timeoutMs,
  }: {
    handleResult: (value: string) => void;
    handlePendingTimers: (nPending: number) => void;
    handleStdout: (value: string) => void;
    handleWindow: (canvas: HTMLCanvasElement, width: number, height: number) => void;
    handleNoWindow: () => void;
    timeoutMs?: number;
  },
): [() => void, Promise<void>] {
  const worker = new TclWorker();
  // Tk draws in the worker, which can only be handed an OffscreenCanvas, and
  // control of a canvas can only be transferred once: hence one per run.
  const canvasEl = document.createElement("canvas");
  const offscreenCanvas = canvasEl.transferControlToOffscreen();
  let openedWindow = false;
  const cancelPromise = Promise.withResolvers<void>();
  const cancel = () => cancelPromise.reject(new Error(CANCELLED));
  let timeout: number | null = null;

  return [
    cancel,
    Promise.race([
      new Promise<void>((res, rej) => {
        const readyPromise = Promise.withResolvers<void>();
        readyPromise.promise.then(() => {
          worker.postMessage({ source, canvas: offscreenCanvas }, [offscreenCanvas]);
        });

        worker.onmessage = ({ data }) => {
          switch (data.type) {
            case "ready":
              readyPromise.resolve();
              break;
            case "result":
              handleResult(data.value);
              // the worker reports a mapped widget before the result, so by now
              // we know whether this run draws at all
              if (!openedWindow) {
                handleNoWindow();
              }
              break;
            case "pending-timers":
              handlePendingTimers(data.value);
              break;
            case "window":
              // an animation keeps scheduling timers, so it never "finishes"
              if (timeout !== null) {
                clearTimeout(timeout);
                timeout = null;
              }
              openedWindow = true;
              handleWindow(canvasEl, data.width, data.height);
              break;
            case "done":
              res();
              break;
            case "error":
              rej(String(data.error));
              break;
            case "stdout":
              handleStdout(data.value);
              break;
            default:
              throw new Error(`unknown event type: ${data.type}`);
          }
        };

        worker.onerror = (error) => rej(error.message || String(error));
      }),

      ...(timeoutMs !== undefined
        ? [
            new Promise<void>((_, rej) => {
              timeout = setTimeout(() => {
                rej(new Error(`timeout: ${timeoutMs}ms`));
              }, timeoutMs);
            }),
          ]
        : []),

      cancelPromise.promise,
    ]).finally(() => {
      worker.terminate();
      if (timeout !== null) {
        clearTimeout(timeout);
      }
    }),
  ];
}

const initialDoc = `proc fib {x} {
  if {$x <= 0} {
    return 0
  }
  if {$x == 1} {
    return 1
  }
  return [expr {[fib [expr {$x - 1}]] + [fib [expr {$x - 2}]]}]
}

fib 8`;

let cancel: (() => void) | null = null;

// closing the window stops the script, the way closing a Tk toplevel does
canvasWindowCloseEl.addEventListener("click", () => {
  closeCanvasWindow();
  cancel?.();
  cancel = null;
  // the script's timers went with it
  showPendingTimers(0);
});
const evaluate = async (code: string) => {
  resultEl.classList.remove("error");
  resultEl.innerText = "...";
  clearStdout();
  try {
    cancel?.();
    let done;
    [cancel, done] = runTcl(code, {
      handleResult(value) {
        resultEl.innerText = value.length ? value : " ";
      },
      handlePendingTimers(nPending) {
        showPendingTimers(nPending);
      },
      handleStdout(value) {
        appendStdout(value);
      },
      handleWindow(canvas, width, height) {
        openCanvasWindow(canvas, width, height);
      },
      handleNoWindow() {
        closeCanvasWindow();
      },
      timeoutMs: 10000,
    });
    await done;
  } catch (e) {
    if (e instanceof Error && e.message === CANCELLED) {
      // a newer run, or the canvas window's close button, replaced this one
      return;
    }
    closeCanvasWindow();
    resultEl.classList.add("error");
    resultEl.innerText = String(e);
  } finally {
    scrollStdoutToEnd();
  }
};

const loadSrc = () => {
  const hash = window.location.hash.substring(1).trim();
  if (!hash.length) {
    return null;
  }
  try {
    return atob(hash);
  } catch {
    return null;
  }
};

const storeSrc = (src: string) => {
  window.location.hash = btoa(src);
};

const view = new EditorView({
  state: EditorState.create({
    doc: loadSrc() ?? initialDoc,
    extensions: [
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      emacsTheme(),
      tcl(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          storeSrc(update.state.doc.toString());
          evaluate(update.state.doc.toString()).catch((e) => console.error(e));
        }
      }),
    ],
  }),
  parent: inputContainerEl,
});

evaluate(view.state.doc.toString()).catch((e) => console.error(e));

const examples = {
  "hello world": `puts "Hello, World!"`,

  fibonacci: `proc fib {x} {
  if {$x <= 0} {
    return 0
  }
  if {$x == 1} {
    return 1
  }
  return [expr {[fib [expr {$x - 1}]] + [fib [expr {$x - 2}]]}]
}

fib 8`,

  canvas: `set width 400
set height 300

canvas .c -width $width -height $height -background #1d1f21
pack .c

set box [.c create rectangle 20 110 140 190 -fill #e2725b]
set dx 4

proc step {} {
  global box dx width

  set coords [.c coords $box]
  set x1 [lindex $coords 0]
  set x2 [lindex $coords 2]

  if {$x2 + $dx > $width || $x1 + $dx < 0} {
    set dx [expr {0 - $dx}]
  }

  .c move $box $dx 0
  after 16 step
}

step`,

  uplevel: `proc do {body while condition} {
  if {$while != "while"} {
    error "required word missing"
  }
  set conditionCmd [list expr $condition]
  while {1} {
    uplevel 1 $body
    if {[uplevel 1 $conditionCmd]} then {
    } else {
      break
    }
  }
}

set i 0
do {
  puts $i
  incr i
} while {$i < 5}`,
};

for (const [name, src] of Object.entries(examples)) {
  const option = document.createElement("option");
  option.value = src;
  option.textContent = name;
  exampleSelectEl.appendChild(option);
}

exampleSelectEl.addEventListener("change", (event) => {
  const value = (event.target as HTMLSelectElement).value;
  view.dispatch({
    changes: {
      from: 0,
      to: view.state.doc.length,
      insert: value,
    },
  });
  exampleSelectEl.selectedIndex = 0;
  exampleSelectEl.blur();
});
