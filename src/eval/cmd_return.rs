use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  match args.get(0) {
    Some(val) => Err(EvalError::ReturnError(val.clone())),
    None => Err(EvalError::ReturnError(Value::none())),
  }
}
