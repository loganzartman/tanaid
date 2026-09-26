use crate::eval::{self, EvalCmdResult, EvalContext};
use crate::eval_error::EvalError;
use crate::event_loop::{EventLoop, EventWait};
use crate::parser::ScriptNode;
use crate::value::Value;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

// flow:
// user creates an Interpreter
// the Interpreter is a long-lived handle that allows for integration with the host event loop
// user creates an EvalContext (can bind variables, register commands)
// user hands over the EvalContext to Interpreter via configure()
// user can run() code with a configure()'d Interpreter, and await the output.
// once the output is awaited, more code can be run().
// meanwhile, an external event loop (winit, etc.) can call and handle step(), which runs the Tcl event loop

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
  pub fn step(&mut self) -> Result<StepResult, EvalError> {
    match &self.state {
      InterpreterState::Init => Ok(StepResult::Wait),
      InterpreterState::Idle(context) => {
        let wait = context.event_loop.borrow_mut().next_wait()?;
        match wait {
          EventWait::Idle => return Ok(StepResult::Wait),
          EventWait::Delay(d) => return Ok(StepResult::WaitDuration(d)),
          EventWait::Ready => self.dispatch_ready_event(),
        }
      }
      InterpreterState::Running(_, _) => self.do_running_work(),
    }
  }

  fn dispatch_ready_event(&mut self) -> Result<StepResult, EvalError> {
    let mut context = match std::mem::replace(&mut self.state, InterpreterState::Init) {
      InterpreterState::Init => {
        return Err(EvalError::Generic(
          "interpreter not configured with EvalContext".to_string(),
        ));
      }
      InterpreterState::Idle(context) => context,
      InterpreterState::Running(event_loop, result_future) => {
        // replace state with previous one to avoid breaking interpreter
        self.state = InterpreterState::Running(event_loop, result_future);
        return Err(EvalError::Generic(
          "interpreter is busy running a script".to_string(),
        ));
      }
    };

    let event_loop = Rc::clone(&context.event_loop);
    let Some(event) = event_loop.borrow_mut().take_ready()? else {
      // possible timer disagreement, try again
      return Ok(StepResult::Again);
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

  fn do_running_work(&mut self) -> Result<StepResult, EvalError> {
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
        let mut cx = Context::from_waker(Waker::noop());
        match result_future.as_mut().poll(&mut cx) {
          Poll::Ready(v) => v,
          Poll::Pending => {
            return match event_loop.borrow_mut().next_wait()? {
              EventWait::Ready => Ok(StepResult::Again),
              EventWait::Delay(duration) => Ok(StepResult::WaitDuration(duration)),
              EventWait::Idle => Ok(StepResult::Wait),
            };
          }
        }
      }
    };

    self.state = InterpreterState::Idle(context);
    Ok(StepResult::Done(result?))
  }

  /// Consume an EvalContext and prepare the interpreter to run scripts.
  pub fn configure(&mut self, context: EvalContext) {
    self.state = InterpreterState::Idle(context);
  }

  /// Start running a script. Errors if the interpreter is already running a script (see is_busy()).
  pub fn start(&mut self, script: &ScriptNode) -> Result<(), EvalError> {
    let mut context = match std::mem::replace(&mut self.state, InterpreterState::Init) {
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
      InterpreterState::Idle(context) => context,
    };

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
    assert!(matches!(interpreter.step(), Ok(StepResult::Again)));

    interpreter
      .start(&parser::parse("set x 1").unwrap())
      .expect("interpreter should still hold its EvalContext");
  }
}
