import { createEffect, createSignal, Show } from "solid-js";
import poweredByUrl from "../powered-by.png";
import stopwatchUrl from "../img/stopwtch.webp";
import stopwatchStaticUrl from "../img/stopwtch-static.webp";
import { CodeEditor } from "./CodeEditor.tsx";
import { ExampleSelect } from "./ExampleSelect.tsx";
import { StartIcon, StopIcon } from "./Icons.tsx";
import { loadExamples } from "./load-examples.ts" with { type: "macro" };
import { OutputView } from "./OutputView.tsx";
import { PixelPerfect } from "./PixelPerfect.tsx";
import { createTclRunner, InterruptedError, type State } from "./tcl/tcl-runner.ts";
import type { Result } from "./tcl/messages.ts";
import { Window } from "./Window.tsx";
import { impossible } from "./impossible.ts";
import { applyCursorStyles } from "./cursor.ts";
import type { TclModifierKeys } from "tanaid-tcl";

const examples = loadExamples();

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
  // oxlint-disable-next-line no-unassigned-vars
  let tkWindowRef!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let editorContainerRef!: HTMLDivElement;

  const [isAuto, setIsAuto] = createSignal(true);
  const [result, setResult] = createSignal<Result | Error | undefined>();
  const [stdout, setStdout] = createSignal<string>("");
  const [pendingTimers, setPendingTimers] = createSignal<number>(0);
  const [source, setSource] = createSignal(loadSrc() ?? "");
  const [tclState, setTclState] = createSignal<State>("idle");
  const [windowOpen, setWindowOpen] = createSignal<boolean>(false);
  const [tkCanvas, setTkCanvas] = createSignal<HTMLCanvasElement | null>(null);

  applyCursorStyles();

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
        setWindowOpen(false);
      })
      .catch((err) => {
        if (err instanceof InterruptedError) {
          setResult(undefined);
        } else {
          setResult(err);
          setWindowOpen(false);
        }
      })
      .finally(() => {
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

  createEffect(windowOpen, (windowOpen) => {
    if (windowOpen && !editorContainerRef.contains(document.activeElement)) {
      tkWindowRef.focus();
    }
  });

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
    setWindowOpen(false);
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

  const keysDown = new Map<string, [string, number, TclModifierKeys]>();

  const unPressPressedKeys = () => {
    for (const [key, location, modifiers] of keysDown.values()) {
      runner.handleKeyEvent({
        eventType: "up",
        key,
        location,
        modifiers,
      });
    }
    keysDown.clear();
  };

  let buttonsDown = 0;
  const setButtonsDown = (
    buttons: number,
    event: PointerEvent,
    { x, y }: { x: number; y: number },
  ) => {
    const changed = buttonsDown ^ buttons;
    buttonsDown = buttons;
    const modifiers = modifiersFromEvent(event);
    BUTTON_FOR_BIT.forEach((button, bit) => {
      if (changed & (1 << bit)) {
        const eventType = buttons & (1 << bit) ? "down" : "up";
        runner.handleMouseButtonEvent({ eventType, button, modifiers, x, y });
      }
    });
  };

  return (
    <PixelPerfect>
      <div class="box-border flex size-[round(100%,2px)] flex-col items-center justify-center min-[1200px]:p-6">
        <Window class="size-full max-w-[1200px]" title="tanaid Tcl">
          <div class="flex min-h-0 min-w-0 flex-1 flex-col">
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
                  <ExampleSelect
                    examples={examples}
                    onSelect={(source) => {
                      stop();
                      setSource(source);
                    }}
                  />
                  <div class="flex flex-row">
                    <button class="icon" onClick={handleStartStop}>
                      <Show when={tclState() === "idle"} fallback={<StopIcon />}>
                        <StartIcon />
                      </Show>
                      <div>{tclState() === "idle" ? "Run" : "Stop"}</div>
                    </button>
                  </div>
                  <input
                    type="checkbox"
                    id="checkbox-auto"
                    checked={isAuto()}
                    onChange={(event) => setIsAuto(event.currentTarget.checked)}
                  />
                  <label for="checkbox-auto">Auto-run</label>
                </div>
              </div>
              <div class="flex-1" />
              <a target="_blank" href="https://github.com/loganzartman/tanaid">
                <img alt="powered by tanaid" src={poweredByUrl} class="float-right w-22" />
              </a>
            </div>
            <div
              ref={editorContainerRef}
              class="sunken-panel mb-(--element-spacing) min-h-48 flex-1 overflow-hidden *:h-full"
            >
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
          </div>
        </Window>
        <Window
          ref={tkWindowRef}
          draggable
          title="tanaid-tk"
          open={windowOpen()}
          onClose={() => {
            setIsAuto(false);
            stop();
          }}
          onKeyDown={(event) => {
            if (event.key !== "Tab") {
              event.preventDefault();
              event.stopPropagation();
            }
            const { key, location } = event;
            const modifiers = modifiersFromEvent(event);
            keysDown.set(event.code, [key, location, modifiers]);
            runner.handleKeyEvent({ eventType: "down", key, location, modifiers });
          }}
          onKeyUp={(event) => {
            if (event.key !== "Tab") {
              event.preventDefault();
              event.stopPropagation();
            }
            if (!keysDown.delete(event.code)) {
              // tk did not observe the keydown
              return;
            }
            const { key, location } = event;
            const modifiers = modifiersFromEvent(event);
            runner.handleKeyEvent({ eventType: "up", key, location, modifiers });
          }}
          onPointerDown={(event, position) => {
            (event.currentTarget as Element).setPointerCapture(event.pointerId);
            if (event.isPrimary) {
              setButtonsDown(event.buttons, event, position);
            }
          }}
          onPointerUp={(event, position) => {
            if (event.isPrimary) {
              setButtonsDown(event.buttons, event, position);
            }
          }}
          onPointerMove={(event, position) => {
            runner.handleMouseMotionEvent({ ...position, modifiers: modifiersFromEvent(event) });
            // presses and releases while another button is down arrive as pointermove
            if (event.isPrimary && buttonsDown !== 0) {
              setButtonsDown(event.buttons, event, position);
            }
          }}
          onPointerCancel={(event, position) => {
            if (event.isPrimary) {
              setButtonsDown(0, event, position);
            }
          }}
          onFocusOut={() => {
            unPressPressedKeys();
          }}
        >
          {tkCanvas()}
        </Window>
      </div>
    </PixelPerfect>
  );
}

/** Bit `i` of `PointerEvent.buttons` is the button with this `PointerEvent.button` value. */
const BUTTON_FOR_BIT = [0, 2, 1, 3, 4];

function modifiersFromEvent(event: KeyboardEvent | PointerEvent): TclModifierKeys {
  return {
    alt: event.altKey,
    ctrl: event.ctrlKey,
    meta: event.metaKey,
    shift: event.shiftKey,
  };
}
