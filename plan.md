# event loop refactor plan

goal: enable `bind` in tanaid-tk to receieve keyboard inputs.

```rust
pub trait Event {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut EvalContext,
  ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>>;
}
```

- event loop schedules then FIFOs
  - events: `HashMap<id_monotonic, Box<dyn Event>>`
  - pending: `PriorityQueue<(deadline, id_monotonic)>`
  - due: `Queue<id_monotonic>`
  - pushed events get deadline 0, to put them in front of elapsed timers; roughly matches tcl
- support pushing events to event_loop from integrators
  - waker: `Option<Waker>`
  - expose event_loop on context (as shared Rc)
- `bind` handler matches EvalError::Break and EvalError::Continue
  - break stops propagation

## done

- `Event` trait; `TimerEvent`; single heap keyed `(deadline, id)`
- `event_loop` moved to `Rc<RefCell<EventLoop>>`, clock handle cloned into the loop
- `EventWait` / `next_wait`; `EventWaiter`

## 1. fix waiting

- `EventWait::Until(d)` already holds a *relative delay*, but all three consumers
  subtract the clock again. rename to `EventWait::In(Duration)` and use directly:
  - `context.rs` `wait_for_event` (saturates to 0 -> busy spin; repro below)
  - `main.rs` / `repl.rs` `about_to_wait` (subtracts backwards -> wakes far too late)
  ```tcl
  after 1500 {set a 1}
  vwait a
  after 2000 {set x 1}
  vwait x
  # real 3.51, user 1.50  <- second vwait spins
  ```
- `as_millis()` truncates; use `as_micros().div_ceil(1000)` so a 9.6ms wait doesn't need an extra pass
- `EventWaiter::poll`: `Output = Result<(), EvalError>`, drop the `panic!`
- `wait_for_event` always awaits `EventWaiter`, for every `EventWait` arm.
  today only `Idle` registers a waker, so a pushed key can't preempt a timer wait.
  collapses to:
  ```rust
  pub async fn wait_for_event(&self) -> Result<(), EvalError> {
    EventWaiter::wait(Rc::clone(&self.event_loop)).await
  }
  ```
- a wake means "check again", not "an event is ready". `next_wait` stays the authority;
  spurious wakes are expected (a newly scheduled sooner timer must shorten the wait)
- keep `sleep_ms` as a hook: `after ms` with no script is a plain sleep that must not
  pump events

## 2. hosts drive the computation

evaluating a script needs `&mut EvalContext`, so the in-flight future must borrow or own
the context. that means it cannot live in `EvalContext` (self-referential) or in
`EventLoop` (owned by the context). the context consumes itself into a handle and comes
back when the computation finishes.

```rust
impl EvalContext {
  /// consumes the context; returned in `Step::Done`
  pub fn start(self, script: ScriptNode) -> Running;
}

pub struct Running {
  pending: Option<Pin<Box<dyn Future<Output = (EvalContext, Result<Value, EvalError>)>>>>,
  event_loop: Rc<RefCell<EventLoop>>,   // reachable while the context is not
}

pub enum Step {
  Done(EvalContext, Result<Value, EvalError>),
  Ready,            // more work due now; step again immediately
  Wait(Duration),   // nothing due for this long
  Idle,             // nothing scheduled; only a push can change this
}

impl Running {
  pub fn step(&mut self) -> Result<Step, EvalError>;
}
```

- no `arm_timer` hook. each host wakes itself with what it already has
- `eval_blocking` stays for tests and simple embedders

### winit (tanaid-cli)

replaces `eval_blocking` at `main.rs:77` and `pollster::block_on(poll_event())` at
`main.rs:120` -- both of which are why `vwait` currently freezes the window.

```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
  match self.running.step()? {
    Step::Done(..) => event_loop.exit(),
    Step::Ready => event_loop.set_control_flow(ControlFlow::Poll),
    Step::Wait(d) => event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + d)),
    Step::Idle => event_loop.set_control_flow(ControlFlow::Wait),
  }
}
```

- drop `with_std_time`'s `std::thread::sleep`
- drop `.expect("clock should be configured")` at `main.rs:137` and `repl.rs:160`;
  both call sites already have an error path
- keep the existing rule: idle + never had a window -> exit

### wasm (tanaid-wasm)

replaces `run` / `runEventLoop`. the driver is where `setTimeout` lives.

```rust
fn drive(running: Rc<RefCell<Running>>) {
  match running.borrow_mut().step() {
    Ok(Step::Done(..)) => resolve(),
    Ok(Step::Ready) => set_timeout(move || drive(running), 0),
    Ok(Step::Wait(d)) => set_timeout(move || drive(running), d.as_micros().div_ceil(1000)),
    Ok(Step::Idle) => {}   // parked; a push restarts it
    Err(e) => reject(e),
  }
}
```

- timer future owns its closure and `clearTimeout`s on `Drop`; delete `timeout_ids`,
  `clear_timeout`, and the struct-level `#[expect(dead_code)]`
- delete `#[derive(Clone)]` on `Interpreter` (unused; extra aliasing route)
- `busy: Cell<bool>` checked at the top of each export -> `JsError("interpreter is busy")`
  instead of a `BorrowMutError` panic
- propagate the `next_event_delay` / `with_clock_monotonic_us` renames (crate does not
  currently build)

## 3. bind

- `TkContext` gains:
  ```rust
  bindings: Rc<RefCell<HashMap<(String, Pattern), String>>>,  // template, unsubstituted
  event_loop: Rc<RefCell<EventLoop>>,                         // from Tk::install
  ```
- `Pattern` parsed at `bind` time, not matched as a string:
  ```rust
  #[derive(Hash, Eq, PartialEq)]
  pub struct Pattern {
    kind: EventKind,         // KeyPress | KeyRelease
    modifiers: Modifiers,
    detail: Option<String>,  // keysym; None = any
  }
  ```
- `cmd/bind.rs` registered like `pack`
- `KeyEvent` implements `Event`; winit `WindowEvent::KeyboardInput` pushes it immediate
- keysym table: winit `Key`/`NamedKey` -> tk names (`Return`, `Escape`, `space`, ...).
  partial coverage shows up as bindings that silently never fire
- focus: with one packed widget the target is that widget. needs a real answer later
- dispatch fires **one script per binding tag**, in tag order -- not one script total
  - tags, minimal: widget path, then `all` (skip class/toplevel/`bindtags` for now)
  - within a tag, most specific pattern only: `<Key-a>` beats `<KeyPress>`
  - `EvalError::BreakError` stops remaining tags; `ContinueError` skips to the next
  - substitute `%K` `%A` `%W` and `%%` textually, then `parse_script_caching`, then
    `eval_returnable_script` at `GLOBAL_FRAME`
- text substitution, not variables: tk substitutes before parsing, so a value can change
  word structure. variables would be a different language, not a subset

## 4. tests

- restore `fires_timers_in_deadline_order` and `handles_nested_and_cancelled_timers`
- pushed (immediate) event ordering vs an elapsed timer
- `after cancel` on an unknown/already-fired id
- `bind` substitution: `%K`, `%A`, `%W`, literal `%%`
- `bind` tag order: widget then `all`; `break` stops the second

## out of scope

- `bgerror` -- an error in a bind script aborts the event loop, same as a timer today
- `after idle`. note on `cmd_after.rs:35`: implementing it needs a ready/pending split,
  because an idle event must sort after *elapsed* timers but before *pending* ones, and
  that boundary is `now`, which no insertion-time key can track
- stdin as an event source
- `bindtags`, widget classes, non-key events
