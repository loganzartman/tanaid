use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_val, assign_vars @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lassign listVal ?varName ...?".to_string(),
    ));
  };

  let list_val_list = list_val.repr_list()?;

  let mut list_iter = list_val_list.iter();
  for assign_var in assign_vars {
    context.set_variable(
      frame,
      assign_var.repr_str()?,
      list_iter.next().cloned().unwrap_or(Value::none()),
    );
  }

  let rest = list_iter.cloned().collect::<Vec<_>>();

  Ok(Value::from(rest))
}
