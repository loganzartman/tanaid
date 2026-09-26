use std::{
  sync::Arc,
  task::{Wake, Waker},
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

pub fn run_blocking(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut interpreter = Interpreter::new();
  interpreter.configure(std::mem::replace(context, EvalContext::new()));
  interpreter.start(script)?;

  let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));

  let result;
  loop {
    match interpreter.step(&waker) {
      Ok(StepResult::Again) => continue,
      Ok(StepResult::WaitDuration(duration)) => std::thread::park_timeout(duration),
      Ok(StepResult::Wait) => std::thread::park(),
      Ok(StepResult::Done(value)) => {
        result = Ok(value);
        break;
      }
      Err(err) => {
        result = Err(err);
        break;
      }
    }
  }

  *context = interpreter.take_context()?;
  result
}

#[cfg(test)]
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
