import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { tcl } from "@sourcebot/codemirror-lang-tcl";
import { vscodeTheme } from "./theme.ts";
import TclWorker from "./tcl.worker.ts?worker";

function runTcl(
  source: string,
  { timeoutMs }: { timeoutMs?: number } = {},
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

const inputContainerEl = document.getElementById("input")! as HTMLDivElement;
const outputEl = document.getElementById("output")! as HTMLPreElement;

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
  try {
    cancel?.();
    let result;
    [cancel, result] = runTcl(code, { timeoutMs: 2000 });
    outputEl.innerText = await result;
  } catch (e) {
    outputEl.innerText = String(e);
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
      keymap.of([...defaultKeymap, ...historyKeymap]),
      vscodeTheme(),
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
