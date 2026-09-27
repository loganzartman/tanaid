use std::{
  sync::Arc,
  task::{Wake, Waker},
  time::Duration,
};

use crate::{
  eval::EvalContext,
  eval_error::EvalError,
  interpreter::{Interpreter, StepResult},
  parser::ScriptNode,
  value::Value,
};

/// Wake by unparking the parked thread
struct ThreadWaker(std::thread::Thread);
impl Wake for ThreadWaker {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}

/// Run a script using a blocking wait(Duration) function, as well as a Waker that can interrupt it.
pub fn run_with_blocking_wait(
  script: &ScriptNode,
  context: &mut EvalContext,
  waker: &Waker,
  mut wait: impl FnMut(Option<Duration>) -> Result<(), EvalError>,
) -> Result<Value, EvalError> {
  let mut interpreter = Interpreter::new();
  interpreter.configure(std::mem::replace(context, EvalContext::new()));
  interpreter.start(script)?;

  let result = (|| -> Result<Value, EvalError> {
    loop {
      match interpreter.step(waker)? {
        StepResult::Again => continue,
        StepResult::WaitDuration(duration) => wait(Some(duration)).map_err(context_lost_error)?,
        StepResult::Wait => wait(None).map_err(context_lost_error)?,
        StepResult::Done(value) => {
          return Ok(value);
        }
      };
    }
  })();

  if let Ok(restored) = interpreter.take_context() {
    *context = restored;
  }
  result
}

fn context_lost_error(error: EvalError) -> EvalError {
  EvalError::Generic(format!("Error while waiting (context lost): {}", error))
}

/// Run a script, blocking the current thread until it's done.
pub fn run_blocking(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
  let wait = move |duration: Option<Duration>| {
    match duration {
      None => std::thread::park(),
      Some(d) => std::thread::park_timeout(d),
    }
    Ok(())
  };
  run_with_blocking_wait(script, context, &waker, wait)
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
  use std::assert_matches;

  use super::*;
  use crate::{
    eval::{EvalContext, GLOBAL_FRAME},
    parser,
  };

  #[test]
  fn runs_sync() -> Result<(), Box<dyn std::error::Error>> {
    let script = parser::parse("expr {2 + 2}")?;
    let mut context = EvalContext::new();
    assert_eq!(run_blocking(&script, &mut context)?.repr_int()?, 4);
    Ok(())
  }

  #[test]
  fn runs_async() -> Result<(), Box<dyn std::error::Error>> {
    let script = parser::parse("update; expr 1;")?;
    let mut context = EvalContext::new();
    assert_eq!(run_blocking(&script, &mut context)?.repr_int()?, 1);
    Ok(())
  }

  #[test]
  fn runs_async_timer() -> Result<(), Box<dyn std::error::Error>> {
    let script = parser::parse("after 10 {set x 1}; vwait x; return $x;")?;
    let mut context = EvalContext::new().with_std_time();
    assert_eq!(run_blocking(&script, &mut context)?.repr_int()?, 1);
    Ok(())
  }

  #[test]
  fn preserves_context_on_error() -> Result<(), Box<dyn std::error::Error>> {
    let script = parser::parse("undefined_command")?;
    let mut context = EvalContext::new().with_std_time();
    context.set_variable(GLOBAL_FRAME, "test", Value::from(1));
    assert_matches!(run_blocking(&script, &mut context), Err(_));
    assert!(context.get_variable(GLOBAL_FRAME, "test").is_some());
    Ok(())
  }
}
