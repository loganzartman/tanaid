use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  if !args.is_empty() {
    return Err(EvalError::ArgumentError(
      "continue expects no arguments".to_string(),
    ));
  }
  Err(EvalError::ContinueError)
}
