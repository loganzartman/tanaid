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

  let mut result = vec![];

  let assign_vars_len = assign_vars.len();
  for (i, assign_var) in assign_vars.iter_mut().enumerate() {
    let list_index = 0.max(list_val_list.len() - assign_vars_len) + i;
    let val = list_val_list
      .get(list_index)
      .cloned()
      .unwrap_or(Value::none());
    result.push(val.clone());
    context.set_variable(frame, assign_var.repr_str()?, val.clone());
  }

  Ok(Value::from(result))
}
