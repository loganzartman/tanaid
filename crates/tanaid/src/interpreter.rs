use crate::eval::{self, EvalCmdResult, EvalContext};
use crate::eval_error::EvalError;
use crate::event_loop::{EventAction, EventLoop};
use crate::parser::ScriptNode;
use crate::value::Value;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// Interpreter is a long-lived handle that allows running Tcl code with an event loop.
/// It's designed for integration into a host event loop (e.g. winit or JS).
///
/// If you simply want to get the result of a Tcl script, you may find [run_blocking] more convenient.
///
/// Interpreter consumes an EvalContext via `configure()`, and can then run a script:
///
/// ```rust
/// use tanaid::parser;
/// use tanaid::interpreter::Interpreter;
/// use tanaid::eval::EvalContext;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let script = parser::parse("expr {2 + 2}")?;
/// let mut context = EvalContext::new();
/// let mut interpreter = Interpreter::new();
/// interpreter.configure(context);
/// interpreter.start(&script);
/// # Ok(())
/// # }
/// ```
///
/// While an interpreter is running, starting another script is an error. [Self::is_busy()] will return true.
///
/// You must call [Self::step()] until it returns [StepResult::Done]. [Self::is_busy()] will then return false.
pub struct Interpreter {
  state: InterpreterState,
}

enum InterpreterState {
  Init,
  Idle(EvalContext),
  Running(
    Rc<RefCell<EventLoop>>,
    Pin<Box<dyn Future<Output = (EvalContext, EvalCmdResult)>>>,
  ),
}

pub enum StepResult {
  /// The host should call step() again without delay
  Again,
  /// The host should wait the given duration
  WaitDuration(Duration),
  /// The host should wait indefinitely
  Wait,
  /// The interpreter finished running and yielded a result. The host should wait indefinitely.
  Done(Value),
}

impl Interpreter {
  pub fn new() -> Self {
    Interpreter {
      state: InterpreterState::Init,
    }
  }

  /// Returns true if the interpreter is currently running a script.
  pub fn is_busy(&self) -> bool {
    matches!(self.state, InterpreterState::Running(_, _))
  }

  /// Do work if there's work to be done, returning a result indicating what to do next.
  pub fn step(&mut self, waker: &Waker) -> Result<StepResult, EvalError> {
    let Some(event_loop) = self.event_loop() else {
      return Ok(StepResult::Wait);
    };

    event_loop.borrow_mut().start_step();
    let result = self.do_step(waker);
    event_loop.borrow_mut().end_step(waker);
    result
  }

  fn do_step(&mut self, waker: &Waker) -> Result<StepResult, EvalError> {
    match &self.state {
      InterpreterState::Init => Ok(StepResult::Wait),
      InterpreterState::Idle(context) => {
        let action = context.event_loop.borrow_mut().next_action()?;
        match action {
          EventAction::Idle => Ok(StepResult::Wait),
          EventAction::WaitDuration(d) => Ok(StepResult::WaitDuration(d)),
          EventAction::Ready => self.poll_event(),
        }
      }
      InterpreterState::Running(_, _) => self.do_running_work(waker),
    }
  }

  fn poll_event(&mut self) -> Result<StepResult, EvalError> {
    let mut context = self
      .take_context()
      .expect("poll_event should only be called while Idle");

    let event_loop = Rc::clone(&context.event_loop);
    let event = match event_loop.borrow_mut().take_event() {
      Ok(Some(event)) => event,
      Ok(None) => {
        // possible timer disagreement, try again
        self.state = InterpreterState::Idle(context);
        return Ok(StepResult::Again);
      }
      Err(e) => {
        self.state = InterpreterState::Idle(context);
        return Err(e);
      }
    };

    self.state = InterpreterState::Running(
      Rc::clone(&event_loop),
      Box::pin(async move {
        let result = event.dispatch(&mut context).await.map(|_| Value::none());
        (context, result)
      }),
    );

    Ok(StepResult::Again)
  }

  fn do_running_work(&mut self, waker: &Waker) -> Result<StepResult, EvalError> {
    let (context, result) = match &mut self.state {
      InterpreterState::Init => {
        return Err(EvalError::Generic(
          "interpreter not configured with EvalContext".to_string(),
        ));
      }
      InterpreterState::Idle(_) => {
        return Err(EvalError::Generic(
          "interpreter is not running anything".to_string(),
        ));
      }
      InterpreterState::Running(event_loop, result_future) => {
        let mut cx = Context::from_waker(waker);
        match result_future.as_mut().poll(&mut cx) {
          Poll::Ready(v) => v,
          Poll::Pending => {
            return match event_loop.borrow_mut().next_action()? {
              EventAction::Ready => Ok(StepResult::Again),
              EventAction::WaitDuration(duration) => Ok(StepResult::WaitDuration(duration)),
              EventAction::Idle => Ok(StepResult::Wait),
            };
          }
        }
      }
    };

    self.state = InterpreterState::Idle(context);
    Ok(StepResult::Done(result?))
  }

  fn event_loop(&self) -> Option<Rc<RefCell<EventLoop>>> {
    match &self.state {
      InterpreterState::Init => None,
      InterpreterState::Idle(context) => Some(Rc::clone(&context.event_loop)),
      InterpreterState::Running(event_loop, _) => Some(Rc::clone(event_loop)),
    }
  }

  /// Consume an EvalContext and prepare the interpreter to run scripts.
  pub fn configure(&mut self, context: EvalContext) {
    self.state = InterpreterState::Idle(context);
  }

  /// Return the context if the interpreter is idle. Otherwise, return an error.
  pub fn take_context(&mut self) -> Result<EvalContext, EvalError> {
    match std::mem::replace(&mut self.state, InterpreterState::Init) {
      InterpreterState::Init => {
        self.state = InterpreterState::Init;
        return Err(EvalError::Generic(
          "interpreter not configured with EvalContext".to_string(),
        ));
      }
      InterpreterState::Running(event_loop, result_future) => {
        // replace state with previous one to avoid breaking interpreter
        self.state = InterpreterState::Running(event_loop, result_future);
        return Err(EvalError::Generic(
          "interpreter is busy running a script".to_string(),
        ));
      }
      InterpreterState::Idle(context) => Ok(context),
    }
  }

  /// Start running a script. Errors if the interpreter is already running a script (see is_busy()).
  pub fn start(&mut self, script: &ScriptNode) -> Result<(), EvalError> {
    let mut context = self.take_context()?;

    let script = script.clone();
    self.state = InterpreterState::Running(
      Rc::clone(&context.event_loop),
      Box::pin(async move {
        let result = eval::eval(&script, &mut context).await;
        (context, result)
      }),
    );

    Ok(())
  }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
  use super::*;
  use crate::eval::GLOBAL_FRAME;
  use crate::event_loop::Event;
  use crate::parser;
  use std::cell::Cell;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::task::Wake;

  struct NoopEvent;

  impl Event for NoopEvent {
    fn dispatch<'a>(
      self: Box<Self>,
      _ctx: &'a mut EvalContext,
    ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>> {
      Box::pin(async { Ok(()) })
    }
  }

  #[derive(Default)]
  struct CountingWaker(AtomicUsize);

  impl CountingWaker {
    fn wakes(&self) -> usize {
      self.0.load(Ordering::Relaxed)
    }
  }

  impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
      self.0.fetch_add(1, Ordering::Relaxed);
    }
  }

  /// a context whose clock only moves when the test moves it, or when a script calls
  /// `work ms`, which stands in for a computation that takes that long
  fn context_with_manual_clock() -> (EvalContext, Rc<Cell<Duration>>) {
    let clock = Rc::new(Cell::new(Duration::ZERO));
    let context_clock = Rc::clone(&clock);
    let mut context = EvalContext::new().with_clock_monotonic(move || context_clock.get());

    let work_clock = Rc::clone(&clock);
    context.register_command("work", move |args, _context, _frame| {
      let ms = args[0].repr_int()?.saturating_cast::<u64>();
      work_clock.set(work_clock.get() + Duration::from_millis(ms));
      Ok(Value::none())
    });

    (context, clock)
  }

  fn start(context: EvalContext, source: &str) -> Interpreter {
    let mut interpreter = Interpreter::new();
    interpreter.configure(context);
    interpreter.start(&parser::parse(source).unwrap()).unwrap();
    interpreter
  }

  #[test]
  fn step_returns_when_handler_outlasts_its_timer() {
    // like an animation whose frame takes longer than its interval: the next timer is
    // already due whenever the handler returns
    let (context, _clock) = context_with_manual_clock();
    let mut interpreter = start(
      context,
      "set n 0
       proc frame {} {
         global n done
         after 16 frame
         work 32
         if {[incr n] == 3} {set done 1}
       }
       frame
       vwait done",
    );

    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Done(_))
    ));
  }

  #[test]
  fn step_dispatches_quick_events_together() {
    let (context, _clock) = context_with_manual_clock();
    let mut interpreter = start(
      context.with_yield_after(Duration::from_millis(10)),
      "after 0 {set a 1}; after 0 {set b 1}; after 0 {set done 1}; vwait done",
    );

    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Done(_))
    ));
  }

  #[test]
  fn step_yields_once_per_wait_when_yield_after_is_zero() {
    let (context, _clock) = context_with_manual_clock();
    let mut interpreter = start(
      context.with_yield_after(Duration::ZERO),
      "after 0 {set a 1}; after 0 {set done 1}; vwait done",
    );

    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Done(_))
    ));
  }

  #[test]
  fn step_doesnt_wake_for_events_scheduled_while_stepping() {
    let (context, clock) = context_with_manual_clock();
    let wakes = Arc::new(CountingWaker::default());
    let waker = Waker::from(Arc::clone(&wakes));
    let mut interpreter = start(context, "proc tick {} {after 10 tick}; tick; vwait forever");

    // the first step leaves the waker stored. the second dispatches a tick, which schedules
    // the next one.
    assert!(matches!(
      interpreter.step(&waker),
      Ok(StepResult::WaitDuration(_))
    ));
    clock.set(Duration::from_millis(10));
    assert!(matches!(
      interpreter.step(&waker),
      Ok(StepResult::WaitDuration(_))
    ));

    assert_eq!(wakes.wakes(), 0);
  }

  #[test]
  fn step_wakes_host_when_event_is_pushed_between_steps() {
    let (context, _clock) = context_with_manual_clock();
    let event_loop = Rc::clone(&context.event_loop);
    let wakes = Arc::new(CountingWaker::default());
    let waker = Waker::from(Arc::clone(&wakes));
    let mut interpreter = start(context, "vwait forever");

    assert!(matches!(interpreter.step(&waker), Ok(StepResult::Wait)));
    event_loop.borrow_mut().push_immediate(Box::new(NoopEvent));

    assert_eq!(wakes.wakes(), 1);
  }

  #[test]
  fn step_wakes_host_after_script_ends_or_fails() {
    for source in ["set x 1", "undefined_command"] {
      let (context, _clock) = context_with_manual_clock();
      let event_loop = Rc::clone(&context.event_loop);
      let wakes = Arc::new(CountingWaker::default());
      let waker = Waker::from(Arc::clone(&wakes));
      let mut interpreter = start(context, source);

      let _ = interpreter.step(&waker);
      assert!(!interpreter.is_busy());
      event_loop.borrow_mut().push_immediate(Box::new(NoopEvent));

      assert_eq!(wakes.wakes(), 1, "after `{source}`");
    }
  }

  #[test]
  fn step_waits_out_sleep_when_event_is_due() {
    let (context, clock) = context_with_manual_clock();
    let mut interpreter = start(context, "after 50 {set x 1}; after 1000");

    // the timer can't run until the script stops sleeping, so it's no reason to step sooner
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::WaitDuration(d)) if d == Duration::from_millis(1000)
    ));
    clock.set(Duration::from_millis(50));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::WaitDuration(d)) if d == Duration::from_millis(950)
    ));
    clock.set(Duration::from_millis(1000));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Done(_))
    ));

    // the timer outlived the sleep, and the idle interpreter runs it
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Done(_))
    ));
    let context = interpreter.take_context().unwrap();
    assert!(context.get_variable(GLOBAL_FRAME, "x").is_some());
  }

  #[test]
  fn step_keeps_context_when_ready_event_disappears() {
    // the clock runs backwards after some number of reads. for one of these numbers,
    // next_action's read says the timer is due and take_event's says it isn't.
    let mut disagreed = false;
    for late_reads in 1..=6 {
      let reads = Cell::new(0);
      let context = EvalContext::new().with_clock_monotonic(move || {
        reads.set(reads.get() + 1);
        if reads.get() <= late_reads {
          Duration::from_millis(10)
        } else {
          Duration::ZERO
        }
      });
      context
        .event_loop
        .borrow_mut()
        .push_scheduled(Box::new(NoopEvent), Duration::from_millis(5));

      let mut interpreter = Interpreter::new();
      interpreter.configure(context);
      let result = interpreter.step(Waker::noop());

      // ready, but nothing was taken
      if matches!(result, Ok(StepResult::Again)) && !interpreter.is_busy() {
        disagreed = true;
        interpreter
          .start(&parser::parse("set x 1").unwrap())
          .expect("interpreter should still hold its EvalContext");
      }
    }
    assert!(disagreed);
  }
}
