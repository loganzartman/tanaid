# Review: `loganz/webgfx` against `main`

- **Commit reviewed:** `68bc46a` (14 commits, 37 files, about +1750/-730). File and line references below are for this commit.
- **Date:** 2026-10-08
- **Source:** an automated review agent working from a read-only copy of the branch. Lines marked "Spot-checked" were confirmed separately, by reading the code or running the command.
- **Browser checks:** run in headless Chromium against the local dev server. Its GPU is a software fallback, so the GPU timings here say nothing about real hardware.

## Verdict

Not safe to merge as it stands. The event loop refactor is sound: its five design intentions hold, the tests pin them, and the browser hang it targets is fixed. The problems are in the playground rewrite.

The two that matter most are finding 1 (a script can no longer be killed) and finding 2 (`pnpm lint` fails, so CI goes red). Findings 3 and 4 are regressions from `main` with small fixes.

## Checklist

- [ ] 1. Non-yielding or crashing scripts wedge the runner; a printing loop freezes the tab
- [ ] 2. `pnpm lint` fails
- [ ] 3. stdout is never cleared between runs
- [ ] 4. `abortSignal` is required at runtime but typed optional
- [ ] 5. Every run requires and waits for a new WebGPU renderer
- [ ] 6. `wasWindowOpen` is never assigned; stale window state leaks into the next run
- [ ] 7. Abort leaves the interpreter busy
- [ ] 8. `run()` is not safe while the worker is still loading
- [ ] 9. A continuously ready event loop gets about 20% of wall time in the browser
- [ ] 10. A Tk script that returns without `vwait` shows nothing in the browser

## Findings, most severe first

### 1. Non-yielding or crashing scripts wedge the runner; a printing loop freezes the tab

- **Where:** `packages/playground/src/tcl/tcl-runner.ts:72-96` and `:157`, `packages/playground/src/tcl/tcl.worker.ts:44-46`, `packages/playground/src/App.tsx:101`
- **Defect:** Stop only posts a message and awaits a `result` that may never come. There is no `terminate()`, no timeout and no worker `error` listener, and stdout is posted once per `puts`. `main` ran each script in a fresh worker with `terminate()`, a 10 s timeout, `worker.onerror` and batched stdout. Because the app runs the source on every keystroke, half-typed loops hit this.
- **Scenario a:** `while {1} {}`. After Stop, the button stays on the stop icon. Replacing the source with `expr {1 + 1}` never runs. `after 100 {set go 1}; while {![info exists stop]} {update}` behaves the same, because `update` never yields by design.
- **Scenario b:** `proc f {} {f}; f`. The worker throws `RangeError: Maximum call stack size exceeded` and sends no `result`. The next edit sends `stop`, the wasm panics with `RefCell already borrowed`, and the runner is still stuck 12 s later.
- **Scenario c:** `while {1} {puts hi}`. The page's main thread did not respond for the 15 s it was watched. Reloading runs the same script again from the URL hash.
- **Checked by:** all three run in headless Chromium. Spot-checked: `main`'s runner has `terminate()`, a timeout and `onerror`, and this branch's has none.
- **Confidence:** high.
- **Fix direction:** see "Going back to `terminate()`" below.

### 2. `pnpm lint` fails

- **Where:** `packages/playground/src/impossible.ts:2`
- **Defect:** `${x}` with `x: never` trips `typescript(restrict-template-expressions)`.
- **Scenario:** `.github/workflows/ci.yml:50` runs `pnpm lint`, so the node job fails. `oxfmt --check` passes when run on its own.
- **Checked by:** `pnpm lint` run by the reviewer. Spot-checked: run again, same error.
- **Confidence:** high.

### 3. stdout is never cleared between runs

- **Where:** `packages/playground/src/App.tsx:71-73`
- **Defect:** `onStdout` only appends. Nothing resets `stdout` or `result` when a run starts. `main` reset both.
- **Scenario:** load `puts hello`, then type two spaces, which causes two more runs. The stdout pane shows `hello` three times, and it grows with every keystroke.
- **Checked by:** run in headless Chromium. Spot-checked by reading: `setStdout` is only ever called to append.
- **Confidence:** high.

### 4. `abortSignal` is required at runtime but typed optional

- **Where:** `crates/tanaid-wasm/src/tcl.rs:45-47`
- **Defect:** the field has `#[tsify(optional)]` but no serde default.
- **Scenario:** `tcl.run("expr 1", { handleEventLoopStatus() {} })` and `tcl.run("expr 1", {})` both reject with ``failed to parse options: ... missing field `abortSignal` ``. Only a call with no options, or with options that include a signal, works. This breaks existing `tanaid-tcl` callers.
- **Checked by:** run against the built wasm. Spot-checked by reading: `handle_event_loop_status` has `serde(default)` and `abort_signal` does not.
- **Confidence:** high.

### 5. Every run requires and waits for a new WebGPU renderer

- **Where:** `packages/playground/src/tcl/tcl.worker.ts:49-51`
- **Defect:** `attachCanvas` is awaited before `tcl.run` on every run, including for scripts that never use Tk.
- **Scenario, no adapter:** with WebGPU disabled, `puts hello; expr {6 * 7}` gives `Failed to create renderer: Couldn't find suitable device` and no output. `vite.config.ts` still targets firefox134 and safari18.2.
- **Scenario, fallback adapter:** on the software GPU, every run after the first took 2.0 to 6.6 s to return, even for a parse error.
- **Checked by:** both run in headless Chromium. The cost on a hardware GPU was not measured.
- **Confidence:** high for the failure without an adapter; unknown for the cost on real hardware.

### 6. `wasWindowOpen` is never assigned; stale window state leaks into the next run

- **Where:** `packages/playground/src/tcl/tcl.worker.ts:53-66`, `packages/playground/src/App.tsx:94-97` and `:206`
- **Defect:** the worker compares against `wasWindowOpen` but never updates it, so it posts `window-change` on every frame. App resets `windowOpen` when the run's promise settles, and for an interrupted run that happens before the worker has stopped.
- **Scenario:** with `bounce.tcl` running, replace the source with `after 3000` followed by `expr 1`. In 3 of 8 trials the Tk window stayed displayed for the whole non-Tk run. Separately, 96 `window-change` messages were counted in 2 s.
- **Related:** this is the same mechanism as the "timers" readout staying at 1 after Stop. Worker messages carry no run id, so late ones are applied to the next run.
- **Checked by:** run in headless Chromium. Spot-checked by reading: `wasWindowOpen` is declared and read but never assigned.
- **Confidence:** high on the mechanism; how often it shows depends on timing.

### 7. Abort leaves the interpreter busy

- **Where:** `crates/tanaid-wasm/src/tcl.rs:137-141`
- **Defect:** on abort the poll closure returns an error without ending the running future. From reading, the same applies when the status callback or `setTimeout` throws (`:148-153`, `:159`, `:166`).
- **Scenario:** abort `after 100000` and the run rejects with `stopped`. The next `run` on the same `Tcl` rejects with `interpreter is already running a script`. The playground is unaffected because it frees the `Tcl` after each run.
- **Checked by:** run in the browser and natively.
- **Confidence:** high.

### 8. `run()` is not safe while the worker is still loading

- **Where:** `packages/playground/src/tcl/tcl-runner.ts:99-106` and `:136-142`
- **Defect:** state stays `idle` while `runNow` awaits init, and every call overwrites the shared `result` slot.
- **Scenario:** delay the wasm fetch by 1.5 s and type `1`, `2`, `3` after `expr 5`. The editor holds `expr 5123`, the result shows `51`, and only `expr 51` was sent. The other calls reject with `internal error: should be idle`.
- **Checked by:** run on the real page and against a mock worker.
- **Confidence:** high. The window is the worker's load time.

### 9. A continuously ready event loop gets about 20% of wall time in the browser

- **Where:** `crates/tanaid/src/eval/context.rs:76`, `crates/tanaid-wasm/src/tcl.rs:158-161`
- **Defect:** `yield_after` is 1 ms, and each `StepResult::Again` goes through `setTimeout(0)`, which browsers clamp to 4 ms once nested.
- **Scenario:** a self-rescheduling `after 0` timer run for 2 s made 398 steps, with a mean step of 1.02 ms and a mean period of 5.03 ms.
- **Checked by:** measured in the browser.
- **Confidence:** high. This is performance, not correctness.
- **Fix direction:** a larger `yield_after` for the wasm host, or a way to yield that isn't a timer.

### 10. A Tk script that returns without `vwait` shows nothing in the browser

- **Where:** `packages/playground/src/tcl/tcl.worker.ts:55-74` and `:96-100`, compared with `crates/tanaid-cli/src/main.rs:100-107`
- **Defect:** the redraw loop stops as soon as the run resolves. The CLI keeps running while a window is open.
- **Scenario:** `canvas .c -width 64 -height 64; pack .c; .c create rectangle 8 8 40 40` gives result `0`, no `window-change` messages and no window. The samples were given `vwait forever` to work around this.
- **Checked by:** browser side run; CLI side read only.
- **Confidence:** high.

## Going back to `terminate()` (fix direction for finding 1)

The branch keeps one worker alive because the canvas is transferred to it. Checked in headless Chromium:

| Action | Result |
|---|---|
| `transferControlToOffscreen()` a second time on the same element | `InvalidStateError` |
| The same, after the worker holding the canvas was terminated | `InvalidStateError` |
| Posting an `OffscreenCanvas` that already has a context to another thread | `InvalidStateError` |
| `transferControlToOffscreen()` on a new element | works |

So a transferred canvas can't be recovered, but a new `<canvas>` element can be made for each new worker. The runner would own both: create the element, transfer it, hand the element to `App.tsx` through a callback, and on Stop terminate the worker and create the next pair.

- **Cost:** a fresh worker took 27 to 30 ms to post `ready`, over eight tries against the dev server with the 2.7 MB release wasm on localhost and a warm cache. `terminate()` and the init round trip took under 1 ms.
- **Also resolves:** finding 6 and the stale "timers" readout, if messages from any worker other than the current one are ignored. It simplifies finding 8, because the `stopping` state, the queued run and the `stopped` promise are no longer needed.
- **Does not resolve:** scenario c of finding 1. The tab freezes because the main thread receives one message per `puts`, so it never processes the Stop click. stdout needs batching in the worker, as on `main`.
- **Unaffected:** findings 3, 4, 5 and 10.

## Smaller issues and cleanups

- **`crates/tanaid/src/eval/event_loop.rs:84`:** `time + self.yield_after` panics with `overflow when adding durations` for `with_yield_after(Duration::MAX)` once the clock is above zero (a test was run). `saturating_add` avoids it.
- **`crates/tanaid/src/eval/event_loop.rs:211-229`:** `WaitForEvent` ignores its `Context`. After an `Interpreter` step, polling `eval` directly returned `Pending` with no waker registered even though an event was ready (a test was run). An executor that parks until woken, such as `pollster::block_on`, would hang.
- **`crates/tanaid-tk/src/tk_context.rs:112`:** `tk.redraw(0)` after `pack` panics on scale factor validation, and every later `redraw` then throws (run in the browser). `devicePixelRatio` is undefined in workers, so replacing the hard-coded `1` at `tcl.worker.ts:67` with it would pass `NaN`.
- **`Tcl.free()` during a run:** throws `attempted to take ownership of Rust value while it was borrowed` and zeroes the wrapper's pointer. The object lives until the run ends (run in the browser).
- **`crates/tanaid-wasm/src/lib.rs:1`:** the crate-level `cfg` gates `Tcl` as well as `Tk`, so the `tk` feature can't be turned off; without it the build is an empty module. The CI Rust job now compiles nothing from this crate, and `tanaid-tcl` always ships Tk, wgpu and vello.
- **`crates/tanaid-wasm/Cargo.toml:27`:** `AbortSignal` and `EventTarget` are not declared as web-sys features. They arrive through winit.
- **`.vscode/settings.json:12-13`:** switches rust-analyzer to wasm32 with `tk` for everyone, which greys out the CLI and all native-only tests.
- **`crates/tanaid/src/eval/event_loop.rs:93`:** the `Some(current)` branch of `end_step` is dead, because `start_step` always clears the waker. The `Sleep` arm at `:187` is unreachable in practice.
- **`crates/tanaid-wasm/src/tcl.rs:134`:** an early return after the abort listener is added leaves it registered with a dropped closure. Only reachable if `start` fails.
- **`event-loop-status`:** posted on every step with no deduplication, about 200 a second in a busy loop.
- **`packages/playground/src/tcl/tcl-runner.ts:133`:** returns the resolvers object, not the promise. `TclOutcome` is unused.
- **Interrupted runs:** show an empty red result, because `InterruptedError` has no message.

## Tests

- **The event loop design is pinned.** Eleven behaviours were removed one at a time in a scratch copy: keeping the waker in `start_step`; skipping `end_step` always, or only on error; never yielding; yielding on every poll; `>` in place of `>=` for the yield deadline; a stale yield deadline; sorting the sleep marker after events; `next_action` consuming the event; no wake on push; and `update` waiting. Ten made a test fail. The `update` change hung the suite at `eval_update_does_not_fire_future_timer`, so it is caught only by a timeout.
- **One gap:** removing the cleanup in `Sleep::drop` passes all 259 tests.
- **Cases with no test:**
  - a host push during a blocking sleep (behaves correctly in a throwaway test)
  - a blocking `after` inside a handler while another timer is due (behaves correctly in a throwaway test)
  - `TestRunner` with a self-rescheduling `after 0`, which by reading spins forever because the fake clock never passes the yield deadline; not run, and `main` would spin the same way
  - anything in `tanaid-wasm`
  - the runner's state machine
- **A context with no clock:** after a host push, every step returns `EventLoop missing clock_monotonic` indefinitely. `main` did the same.
- **Suites at this commit:** `cargo test --workspace --exclude tanaid-wasm` passes (259 in `tanaid`, 13 in `tanaid-tk`, 1 doctest). `cargo check -p tanaid-wasm --target wasm32-unknown-unknown --features tk` and `cargo fmt --all --check` are clean.

## Checked and found fine

- **The hang fix works in the browser:** with a 20 ms handler on a 1 ms timer the worker stays responsive, and Stop took effect 52 ms after the stop message.
- **The waker:** never called during a step, and stored after every step, including failed ones.
- **The sleep marker:** reports the remaining sleep while a timer is due, and is removed when the interpreter is dropped mid-sleep.
- **Wake-timeout bookkeeping in `tcl.rs`:** stale timeouts are harmless and are cleared on drop.
- **Timer accuracy in the browser:** `after 15` gave a mean period of 15.46 ms.
- **Redraw loop against a run finishing:** guarded by `stopped`.
- **`tk_context.rs`:** attach, resize and `has_window` are fine apart from the scale factor panic above.

## Known before the review

- **Clock origin in the browser:** `clock monotonic` reads `performance.now()` (`crates/tanaid-wasm/src/tcl.rs:76`), which in a worker counts from worker creation, not from the start of the script.

## Not reviewed or not verifiable

- The Tailwind and CSS conversion, visual layout, and dragging in `Window.tsx`.
- Whether the canvas draws correct pixels, and the cost of attaching it on a hardware GPU.
- Firefox and Safari.
- `pnpm build` and the wasm build scripts, which were not run.
- `pnpm-lock.yaml`.
- Native CLI window behaviour. `tanaid-cli` is unchanged on this branch and was only read.
