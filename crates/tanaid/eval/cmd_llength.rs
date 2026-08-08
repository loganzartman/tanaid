use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let [list] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: llength listVal".to_string(),
    ));
  };

  Ok(Value::from(list.repr_list()?.len() as i64))
}
