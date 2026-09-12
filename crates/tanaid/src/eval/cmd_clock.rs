use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};

pub(super) fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  match args {
    [arg] if arg.to_string() == "monotonic" => {
      Ok(Value::from(context.clock_monotonic()?.as_millis() as i64))
    }
    _ => Err(EvalError::ArgumentError(
      "unsupported clock subcommand; expects: clock monotonic".to_string(),
    )),
  }
}
