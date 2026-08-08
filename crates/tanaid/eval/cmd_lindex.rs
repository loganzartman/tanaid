use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let [list_val, index] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lindex listVal index".to_string(),
    ));
  };

  let index_int = index.repr_int()?;
  let Ok(index_usize) = usize::try_from(index_int) else {
    return Ok(Value::none());
  };

  match list_val.repr_list()?.get(index_usize) {
    None => Ok(Value::none()),
    Some(value) => Ok(value.clone()),
  }
}
