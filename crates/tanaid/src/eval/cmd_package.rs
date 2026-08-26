use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  // TODO: Let extensions register package availability instead of hard-coding Tk in core.
  match args {
    [subcommand, arg] if subcommand.to_string() == "require" && arg.to_string() == "Tk" => {
      Ok(Value::none())
    }
    _ => Err(EvalError::ArgumentError(
      "unsupported package command".to_string(),
    )),
  }
}
