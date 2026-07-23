use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::{List, Value};

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_var, vals @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lappend listVar ?value value value ...?".to_string(),
    ));
  };

  let list_var_str = list_var.repr_str()?;
  let mut list_val = match context.get_variable(frame, list_var_str).cloned() {
    Some(list) => list,
    None => Value::from(List::new()),
  };

  let mut list_list = list_val.repr_list()?.as_ref().clone();
  for val in vals {
    list_list.push(val.clone());
  }

  let new_var_val = Value::from(list_list);
  context.set_variable(frame, list_var_str, new_var_val.clone());
  Ok(new_var_val)
}
