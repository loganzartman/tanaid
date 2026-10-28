import { createEffect, createSignal, For, Show } from "solid-js";
import poweredByUrl from "../powered-by.png";
import stopwatchUrl from "../img/stopwtch.webp";
import stopwatchStaticUrl from "../img/stopwtch-static.webp";
import { CodeEditor } from "./CodeEditor.tsx";
import { StartIcon, StopIcon } from "./Icons.tsx";
import { loadExamples } from "./load-examples.ts" with { type: "macro" };
import { Menu, MenuBar, MenuButton } from "./Menu.tsx";
import { OutputView } from "./OutputView.tsx";
import { PixelPerfect } from "./PixelPerfect.tsx";
import { createTclRunner, InterruptedError, type State } from "./tcl/tcl-runner.ts";
import type { Result } from "./tcl/messages.ts";
import { Window } from "./Window.tsx";
import { impossible } from "./impossible.ts";
import { applyCursorStyles } from "./cursor.ts";
import type { TclModifierKeys } from "tanaid-tcl";
import { encodeUrlPayload, type UrlPayload } from "./url.ts";

const examples = loadExamples();

export function App(props: { urlPayload: UrlPayload | null }) {
  // oxlint-disable-next-line no-unassigned-vars
  let tkWindowRef!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let aboutWindowRef!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let editorContainerRef!: HTMLDivElement;

  const [isAuto, setIsAuto] = createSignal(true);
  const [result, setResult] = createSignal<Result | Error | undefined>();
  const [stdout, setStdout] = createSignal<string>("");
  const [pendingTimers, setPendingTimers] = createSignal<number>(0);
  const [source, setSource] = createSignal(() => props.urlPayload?.source ?? "");
  const [tclState, setTclState] = createSignal<State>("idle");
  const [windowOpen, setWindowOpen] = createSignal<boolean>(false);
  const [tkCanvas, setTkCanvas] = createSignal<HTMLCanvasElement | null>(null);
  const [aboutOpen, setAboutOpen] = createSignal(false);

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
      canvas.className = "m-0 touch-none select-none";
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

  createEffect(
    () => ({ source: source() }),
    (payload) => {
      let cancelled = false;

      const timeout = setTimeout(() => {
        encodeUrlPayload(payload)
          .then((encoded) => {
            if (cancelled) {
              return;
            }
            window.history.replaceState(null, "", `#${encoded}`);
          })
          .catch((err) => {
            console.error(err);
          });
      }, 400);

      return () => {
        cancelled = true;
        clearTimeout(timeout);
      };
    },
    { defer: true },
  );

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

  createEffect(aboutOpen, (aboutOpen) => {
    if (aboutOpen) {
      aboutWindowRef.focus();
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
          <div class="m-0 flex">
            <div class="flex flex-col">
              <MenuBar>
                <Menu label="File">
                  <MenuButton
                    onClick={() => {
                      stop();
                      setSource("");
                    }}
                  >
                    New
                  </MenuButton>
                  <MenuButton
                    onClick={() => {
                      const upload = document.createElement("input");
                      upload.type = "file";
                      upload.accept = ".tcl";
                      upload.addEventListener(
                        "change",
                        () => {
                          if (upload.files?.length !== 1) {
                            return;
                          }
                          upload.files[0]
                            .text()
                            .then((text) => {
                              stop();
                              setSource(text);
                            })
                            .catch((e) => setResult(e));
                        },
                        false,
                      );
                      upload.click();
                    }}
                  >
                    Open
                  </MenuButton>
                  <MenuButton
                    onClick={() => {
                      const blob = new Blob([source()], { type: "text/x-tcl" });
                      const url = URL.createObjectURL(blob);
                      const link = document.createElement("a");
                      link.href = url;
                      link.download = "project.tcl";
                      link.click();
                      setTimeout(() => URL.revokeObjectURL(url), 1000);
                    }}
                  >
                    Save
                  </MenuButton>
                </Menu>
                <Menu label="Examples">
                  <For each={Object.entries(examples)}>
                    {([category, sources]) => (
                      <Menu label={category}>
                        <For each={Object.entries(sources)}>
                          {([name, source]) => (
                            <MenuButton
                              onClick={() => {
                                stop();
                                setSource(source);
                              }}
                            >
                              {name}
                            </MenuButton>
                          )}
                        </For>
                      </Menu>
                    )}
                  </For>
                </Menu>
                <Menu label="Help">
                  <MenuButton
                    onClick={() =>
                      window.open(
                        "https://www.magicsplat.com/tcl-docs/doctoc_tcl.html",
                        "_blank",
                        "noopener",
                      )
                    }
                  >
                    Tcl documentation
                  </MenuButton>
                  <MenuButton onClick={() => setAboutOpen(true)}>About tanaid</MenuButton>
                </Menu>
              </MenuBar>
              <div class="mx-(--element-spacing) mt-(--element-spacing) flex flex-row gap-2">
                <button class="icon" onClick={handleStartStop}>
                  <Show when={tclState() === "idle"} fallback={<StopIcon />}>
                    <StartIcon />
                  </Show>
                  <div>{tclState() === "idle" ? "Run" : "Stop"}</div>
                </button>
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
            <a
              class="m-(--element-spacing)"
              target="_blank"
              href="https://github.com/loganzartman/tanaid"
            >
              <img alt="powered by tanaid" src={poweredByUrl} class="float-right w-22" />
            </a>
          </div>
          <div
            ref={editorContainerRef}
            class="sunken-panel mb-0 min-h-48 flex-1 overflow-hidden *:h-full"
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
        </Window>
        <Window
          ref={tkWindowRef}
          draggable
          noContextMenu
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
        <Window
          ref={aboutWindowRef}
          class="min-w-72 min-h-32 text-center"
          draggable
          title="About"
          open={aboutOpen()}
          onClose={() => setAboutOpen(false)}
        >
          <div class="font-bold mt-4">tanaid web playground</div>
          <div>
            <p>
              run a tiny subset of&nbsp;
              <a href="https://www.tcl-lang.org/about/language.html" target="_blank">
                Tcl
              </a>
              &nbsp;in your browser.
            </p>
            <p>
              find more information on{" "}
              <a href="https://github.com/loganzartman/tanaid" target="_blank">
                github
              </a>
              !
            </p>
          </div>
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
