use std::cell::Cell;
use std::rc::Rc;
use std::{task::Waker, time::Duration};

use crate::{
  eval::EvalContext, eval_error::EvalError, parser::ScriptNode,
  run_blocking::run_with_blocking_wait, value::Value,
};

pub struct TestRunner {
  pub context: EvalContext,
  pub clock: Rc<Cell<Duration>>,
}

/// Runs scripts with a fake clock that advances instantly.
impl TestRunner {
  pub fn new() -> Self {
    Self::new_with_context(EvalContext::new())
  }

  pub fn new_with_context(context: EvalContext) -> Self {
    let clock = Rc::new(Cell::new(Duration::ZERO));
    let context_clock = Rc::clone(&clock);

    TestRunner {
      context: context.with_clock_monotonic(move || context_clock.get()),
      clock,
    }
  }

  pub fn run(&mut self, script: &ScriptNode) -> Result<Value, EvalError> {
    let clock = Rc::clone(&self.clock);
    let wait = move |duration: Option<Duration>| {
      match duration {
        None => return Err(EvalError::Generic("test would wait forever".to_string())),
        Some(d) => clock.set(clock.get() + d),
      }
      Ok(())
    };
    run_with_blocking_wait(script, &mut self.context, Waker::noop(), wait)
  }
}

#[cfg(test)]
mod tests {
  use crate::parser;

  use super::*;

  #[test]
  fn runs_timers() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = TestRunner::new();
    let script = parser::parse("after 1000; expr 2 + 2;")?;
    assert_eq!(runner.run(&script)?.repr_int()?, 4);
    assert_eq!(runner.clock.get(), Duration::from_millis(1000));
    Ok(())
  }

  #[test]
  fn doesnt_wait_forever() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = TestRunner::new();
    let script = parser::parse("vwait forever")?;
    let result = runner.run(&script);
    assert!(result.unwrap_err().to_string().contains("wait"));
    Ok(())
  }
}
