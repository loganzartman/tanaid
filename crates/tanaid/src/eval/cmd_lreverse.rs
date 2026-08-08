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
      "wrong number of arguments; expects: lreverse listVal".to_string(),
    ));
  };

  let mut list_list = list.repr_list()?.as_ref().clone();
  list_list.reverse();

  Ok(Value::from(list_list))
}
