use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::parser::WordNode;

pub(super) fn eval(
  words: &[WordNode],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  if !words.is_empty() {
    return Err(EvalError::ArgumentError(
      "continue expects no arguments".to_string(),
    ));
  }
  Err(EvalError::ContinueError)
}
