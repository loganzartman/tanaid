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
        context.event_loop.borrow_mut().start_step();
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
        event_loop.borrow_mut().start_step();
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
  use crate::event_loop::Event;
  use crate::parser;
  use std::cell::Cell;

  struct NoopEvent;

  impl Event for NoopEvent {
    fn dispatch<'a>(
      self: Box<Self>,
      _ctx: &'a mut EvalContext,
    ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>> {
      Box::pin(async { Ok(()) })
    }
  }

  #[test]
  fn step_keeps_context_when_ready_event_disappears() {
    // next_wait's clock read says the timer is due; take_ready's says it isn't
    let reads = Cell::new(0);
    let context = EvalContext::new().with_clock_monotonic(move || {
      reads.set(reads.get() + 1);
      if reads.get() == 1 {
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
    assert!(matches!(
      interpreter.step(Waker::noop()),
      Ok(StepResult::Again)
    ));

    interpreter
      .start(&parser::parse("set x 1").unwrap())
      .expect("interpreter should still hold its EvalContext");
  }
}
