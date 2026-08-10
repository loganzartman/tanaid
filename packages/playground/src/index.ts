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
const stdoutEl = document.getElementById("stdout")! as HTMLElement;

function runTcl(
  source: string,
  { handleStdout, timeoutMs }: { handleStdout: (value: string) => void; timeoutMs?: number },
): [() => void, Promise<string>] {
  const worker = new TclWorker();
  const cancelPromise = Promise.withResolvers<string>();
  const cancel = () => cancelPromise.reject(new Error("cancelled"));
  let timeout: number | null = null;

  return [
    cancel,
    Promise.race([
      new Promise<string>((res, rej) => {
        const readyPromise = Promise.withResolvers<void>();
        readyPromise.promise.then(() => {
          worker.postMessage({ source });
        });

        worker.onmessage = ({ data }) => {
          switch (data.type) {
            case "ready":
              readyPromise.resolve();
              break;
            case "result":
              res(data.value);
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

        worker.onerror = (error) => rej(String(error));
      }),

      ...(timeoutMs !== undefined
        ? [
            new Promise<string>((_, rej) => {
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
const evaluate = async (code: string) => {
  resultEl.innerText = "...";
  stdoutEl.innerText = "";
  try {
    cancel?.();
    let result;
    [cancel, result] = runTcl(code, {
      handleStdout(value) {
        stdoutEl.textContent += value;
      },
      timeoutMs: 2000,
    });
    resultEl.innerText = await result;
  } catch (e) {
    resultEl.innerText = String(e);
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
