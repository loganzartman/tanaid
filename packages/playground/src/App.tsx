import { createEffect, createSignal, Show } from "solid-js";
import poweredByUrl from "../powered-by.png";
import stopwatchUrl from "../img/stopwtch.webp";
import stopwatchStaticUrl from "../img/stopwtch-static.webp";
import { CodeEditor } from "./CodeEditor.tsx";
import { ExampleSelect } from "./ExampleSelect.tsx";
import { loadExamples } from "./load-examples.ts" with { type: "macro" };
import { OutputView } from "./OutputView.tsx";
import { PixelPerfect } from "./PixelPerfect.tsx";
import { createTclRunner } from "./tcl-runner.ts";

const examples = loadExamples();

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

const loadSrc = () => {
  const hash = window.location.hash.substring(1).trim();
  if (!hash.length) {
    return null;
  }
  let binary;
  try {
    binary = atob(hash);
  } catch {
    return null;
  }
  const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    // links from before UTF-8 encoding held Latin-1 characters directly
    return binary;
  }
};

const storeSrc = (src: string) => {
  // `btoa` only accepts Latin-1, so encode the UTF-8 bytes
  const bytes = new TextEncoder().encode(src);
  const binary = Array.from(bytes, (b) => String.fromCharCode(b)).join("");
  window.history.replaceState(null, "", `#${btoa(binary)}`);
};

export function App() {
  const [source, setSource] = createSignal(loadSrc() ?? initialDoc);
  createEffect(source, storeSrc, { defer: true });

  const run = createTclRunner(source, { timeoutMs: 10000 });

  const resultText = () => {
    const outcome = run.outcome();
    switch (outcome.status) {
      case "running":
        return "...";
      case "ok":
        return outcome.value.length ? outcome.value : " ";
      case "error":
        return outcome.message;
    }
  };

  return (
    <PixelPerfect>
      <div class="main-window-container">
        <div
          class="window main-window"
          style={{
            height: "100%",
            "box-sizing": "border-box",
            display: "flex",
            "flex-direction": "column",
          }}
        >
          <div class="title-bar">
            <div class="title-bar-text">tanaid Tcl</div>
          </div>
          <div
            class="window-body"
            style={{ flex: "1", "min-height": "0", display: "flex", "flex-direction": "column" }}
          >
            <div
              style={{
                display: "flex",
                gap: "var(--element-spacing)",
                "margin-bottom": "var(--element-spacing)",
              }}
            >
              <div style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
                <div>
                  run a tiny subset of{" "}
                  <a target="_blank" href="https://www.tcl-lang.org/">
                    Tcl
                  </a>{" "}
                  in your browser
                </div>
                <div>
                  <ExampleSelect examples={examples} onSelect={setSource} />
                </div>
              </div>
              <div style={{ flex: "1" }} />
              <a target="_blank" href="https://github.com/loganzartman/tanaid">
                <img
                  alt="powered by tanaid"
                  src={poweredByUrl}
                  style={{ float: "right", width: "88px" }}
                />
              </a>
            </div>
            <div class="input sunken-panel">
              <CodeEditor value={source()} onChange={setSource} />
            </div>
            <div>
              <div class="status-bar" style={{ width: "100%" }}>
                <div
                  class="status-bar-field"
                  style={{
                    display: "flex",
                    "flex-direction": "row",
                    "align-items": "center",
                    gap: "8px",
                    padding: "8px",
                  }}
                >
                  <label for="result">result:</label>
                  <div
                    id="result"
                    class={["font-mono", { error: run.outcome().status === "error" }]}
                  >
                    {resultText()}
                  </div>
                </div>
                <div
                  class="status-bar-field"
                  style={{
                    "flex-grow": "0",
                    display: "flex",
                    "flex-direction": "column",
                    gap: "8px",
                  }}
                >
                  <div
                    style={{ display: "flex", "flex-direction": "row", "align-items": "center" }}
                  >
                    <Show
                      when={run.pendingTimers() > 0}
                      fallback={<img src={stopwatchStaticUrl} style={{ display: "block" }} />}
                    >
                      <img src={stopwatchUrl} style={{ display: "block" }} />
                    </Show>
                    <div style={{ padding: "8px", "padding-left": "2px" }}>
                      {run.pendingTimers()} timers
                    </div>
                  </div>
                </div>
              </div>
              <div class="field-row-stacked" style={{ "margin-top": "8px" }}>
                <label for="stdout">stdout:</label>
                <div id="stdout" class="stdout sunken-panel">
                  <OutputView text={run.stdout()} />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </PixelPerfect>
  );
}
