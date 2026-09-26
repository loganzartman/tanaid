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
    match interpreter.step(&waker)? {
      StepResult::Again => continue,
      StepResult::WaitDuration(duration) => std::thread::park_timeout(duration),
      StepResult::Wait => std::thread::park(),
      StepResult::Done(value) => {
        result = value;
        break;
      }
    }
  }

  *context = interpreter.take_context()?;
  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{eval::EvalContext, parser};

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
}
