use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::value::{List, Value};

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let mut list = List::new();
  for value in args {
    list.push(value.clone());
  }
  Ok(Value::from(list))
}
