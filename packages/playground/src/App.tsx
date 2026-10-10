import { createEffect, createMemo, createSignal, Show } from "solid-js";
import poweredByUrl from "../powered-by.png";
import stopwatchUrl from "../img/stopwtch.webp";
import stopwatchStaticUrl from "../img/stopwtch-static.webp";
import { CodeEditor } from "./CodeEditor.tsx";
import { ExampleSelect } from "./ExampleSelect.tsx";
import { loadExamples } from "./load-examples.ts" with { type: "macro" };
import { OutputView } from "./OutputView.tsx";
import { createPixelPerfectScale, PixelPerfect } from "./PixelPerfect.tsx";
import { createTclRunner, InterruptedError, type State } from "./tcl/tcl-runner.ts";
import type { Result } from "./tcl/messages.ts";
import { Window } from "./Window.tsx";
import startImg from "../img/start.svg";
import stopImg from "../img/stop.svg";
import { impossible } from "./impossible.ts";
import cursorDarkAuto from "../img/cur-dark.png";
import cursorLightAuto from "../img/cur-light.png";
import { makeCssCursor } from "./cursor.ts";

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
  const [isAuto, setIsAuto] = createSignal(true);
  const [result, setResult] = createSignal<Result | Error | undefined>();
  const [stdout, setStdout] = createSignal<string>("");
  const [pendingTimers, setPendingTimers] = createSignal<number>(0);
  const [source, setSource] = createSignal(loadSrc() ?? initialDoc);
  const [tclState, setTclState] = createSignal<State>("idle");
  const [windowOpen, setWindowOpen] = createSignal<boolean>(false);
  const [tkCanvas, setTkCanvas] = createSignal<HTMLCanvasElement | null>(null);

  const scale = createPixelPerfectScale();
  const cursor = createMemo(() =>
    makeCssCursor({
      lightSrc: cursorLightAuto,
      darkSrc: cursorDarkAuto,
      globalScale: scale().globalScale,
      unitScale: scale().unitScale,
    }),
  );

  const runner = createTclRunner({
    onStdout(value) {
      setStdout((v) => (v + value).slice(-1_000_000));
    },
    onEventLoopStatus(status) {
      setPendingTimers(status.countPending);
    },
    onStateChanged(state) {
      setTclState(state);
    },
    onWindowChanged({ open, canvas }) {
      setWindowOpen(open);
      setTkCanvas(canvas);
      canvas.className = "m-0";
    },
  });

  const runSource = (source: string) => {
    setResult(undefined);
    setStdout("");

    runner
      .run({ source })
      .then((result) => {
        setResult(result);
      })
      .catch((err) => {
        if (err instanceof InterruptedError) {
          setResult(undefined);
        } else {
          setResult(err);
        }
      })
      .finally(() => {
        setWindowOpen(false);
        setPendingTimers(0);
      });
  };

  createEffect(source, storeSrc, { defer: true });
  createEffect(
    () => ({ source: source(), isAuto: isAuto() }),
    ({ source, isAuto }) => {
      if (isAuto) {
        runSource(source);
      }
    },
  );

  const resultText = () => {
    const value = result();
    if (value instanceof Error) {
      return value.message;
    }
    if (value === undefined) {
      return "";
    }
    return value;
  };

  const stop = () => {
    runner.stop();
  };

  const handleStartStop = () => {
    const state = tclState();
    switch (state) {
      case "idle":
        runSource(source());
        break;
      case "running":
        stop();
        break;
      default:
        impossible(state);
    }
  };

  const handleCloseTkWindow = () => {
    stop();
  };

  return (
    <PixelPerfect>
      <div
        class="box-border flex size-[round(100%,2px)] flex-col items-center justify-center min-[1200px]:p-6"
        style={{ cursor: cursor() }}
      >
        <div class="window box-border flex size-full max-w-[1200px] flex-col">
          <div class="title-bar">
            <div class="title-bar-text">tanaid Tcl</div>
          </div>
          <div class="window-body flex min-h-0 flex-1 flex-col">
            <div class="mb-(--element-spacing) flex gap-(--element-spacing)">
              <div class="flex flex-col gap-1">
                <div>
                  run a tiny subset of{" "}
                  <a target="_blank" href="https://www.tcl-lang.org/">
                    Tcl
                  </a>{" "}
                  in your browser
                </div>
                <div class="flex flex-row gap-2">
                  <ExampleSelect examples={examples} onSelect={setSource} />
                  <div class="flex flex-row">
                    <link rel="preload" href={startImg} as="image" />
                    <link rel="preload" href={stopImg} as="image" />
                    <button class="icon" onClick={handleStartStop}>
                      <img src={tclState() === "idle" ? startImg : stopImg} />
                      <div>{tclState() === "idle" ? "Run" : "Stop"}</div>
                    </button>
                  </div>
                  <input
                    type="checkbox"
                    id="checkbox-auto"
                    checked={isAuto()}
                    onChange={(event) => setIsAuto(event.currentTarget.checked)}
                  />
                  <label for="checkbox-auto">Auto</label>
                </div>
              </div>
              <div class="flex-1" />
              <a target="_blank" href="https://github.com/loganzartman/tanaid">
                <img alt="powered by tanaid" src={poweredByUrl} class="float-right w-22" />
              </a>
            </div>
            <div class="sunken-panel mb-(--element-spacing) min-h-48 flex-1 overflow-hidden *:h-full">
              <CodeEditor value={source()} onChange={setSource} />
            </div>
            <div>
              <div class="status-bar w-full">
                <div class="status-bar-field flex flex-row items-center gap-2 p-2">
                  <label for="result">result:</label>
                  <div
                    id="result"
                    class={[
                      "font-mono-13 whitespace-pre-wrap",
                      { "text-error": result() instanceof Error },
                    ]}
                  >
                    {resultText()}
                  </div>
                </div>
                <div class="status-bar-field flex grow-0 flex-col gap-2">
                  <div class="flex flex-row items-center">
                    <Show
                      when={pendingTimers() > 0}
                      fallback={<img src={stopwatchStaticUrl} class="block" />}
                    >
                      <img src={stopwatchUrl} class="block" />
                    </Show>
                    <div class="p-2 pl-0.5">{pendingTimers()} timers</div>
                  </div>
                </div>
              </div>
              <div class="field-row-stacked mt-2">
                <label for="stdout">stdout:</label>
                <div id="stdout" class="sunken-panel h-48 min-h-16 overflow-hidden *:h-full">
                  <OutputView text={stdout()} />
                </div>
              </div>
            </div>
            <Window
              draggable
              title="tanaid-tk"
              open={tclState() === "running" && windowOpen()}
              onClose={() => {
                handleCloseTkWindow();
              }}
            >
              {tkCanvas()}
            </Window>
          </div>
        </div>
      </div>
    </PixelPerfect>
  );
}
