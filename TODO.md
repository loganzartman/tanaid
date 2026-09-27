# TODO

Open issues from the `loganz/keyboard` review.

## Interpreter loses its context when a ready event disappears

`Interpreter::dispatch_ready_event` (`crates/tanaid/src/interpreter.rs`) swaps the state to `Init`
before calling `take_ready()`. If `take_ready()` returns `None` (clock disagreement between
`next_wait()` and `take_ready()`) or an error, the `EvalContext` is dropped. Every later `step()`
returns `Wait` and every `start()` fails with "interpreter not configured".

- Fix: call `take_ready()` before replacing the state, or put `Idle(context)` back on the `None`/`Err` paths.
- Test: `interpreter::tests::step_keeps_context_when_ready_event_disappears` (currently fails).

## CLI exits with code 0 after a script error

In `SourceApp::about_to_wait` (`crates/tanaid-cli/src/main.rs`), the `Err` branch prints the error
and exits the loop but never sets `self.error`, so `run()` returns `Ok` and the process exits 0.

- Fix: store the error in `self.error` (and print to stderr, not stdout).

## CLI keeps running after the window is closed while a script is busy

The exit check in `SourceApp::about_to_wait` requires `!self.interpreter.is_busy()`. A script
blocked in `vwait` (e.g. `sample/tk_vwait.tcl`) keeps the process alive, printing, with no window.
`wish` exits when its main window is destroyed.

- Fix: track whether a window was ever opened (the old `had_window` logic), and exit once it has
  been closed, regardless of `is_busy()`.

## Playground throws on `event_loop_status`

The worker (`packages/playground/src/tcl.worker.ts`) now posts
`{ type: "event_loop_status", countPending }`, but `packages/playground/src/index.ts` still only
handles `"pending-timers"` (reading `data.value`) and throws on any unknown type.

- Fix: rename the case to `"event_loop_status"` and read `data.countPending`.

## wasm host doesn't compile, and busy-loops on `Wait`

`crates/tanaid-wasm/src/lib.rs` still calls `step()` without a waker, and `set_timeout` is now
unused (an error under `warnings = "deny"`). The old loop also treated `StepResult::Wait` as a 0ms
`setTimeout`, spinning the worker at 100% CPU on `vwait forever`.

- Fix: drive the interpreter with `std::future::poll_fn`, passing `cx.waker()` to `step()`:
  - `Done(v)` → `Ready(Ok(v))`
  - `WaitDuration(d)` → `setTimeout(d)` that calls `waker.wake()`, then `Pending`
  - `Again` → `setTimeout(0)` wake (not `wake_by_ref`, which runs as a microtask and starves the worker), then `Pending`
  - `Wait` → `Pending` (an outside source will wake it)
- Call the `event_loop_status` handler before the `match`.

## REPL returns to the prompt before a line finishes

`ReplApp::user_event` (`crates/tanaid-cli/src/repl.rs`) steps a line once, then always sends
`next_tx`, so reedline shows the prompt while a blocking `after` or `vwait` is still running. The
next line fails with "interpreter is busy", and the line's result is never printed.

- Fix: add a `line_running: bool` to `ReplApp`.
  - `ReplEvent::Line`: parse and `start()` only. On error, print it and send `next_tx`; otherwise set `line_running = true`.
  - `about_to_wait`: if `line_running` and `step()` returns `Done` or `Err`, print the value or error, clear the flag, and send `next_tx`.
- The first `Done`/`Err` after `start()` always belongs to the line: events dispatched while it
  waits run inside the line's own future.
- Then delete `run_line` and its `pollster::block_on`.
- Known limitation: Ctrl-C can't interrupt a running line, because reedline isn't reading input.
