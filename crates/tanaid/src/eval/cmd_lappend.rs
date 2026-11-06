use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_var, vals @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lappend listVar ?value value value ...?".to_string(),
    ));
  };

  let list_var_str = list_var.repr_str();
  context.update_variable(frame, list_var_str, |val| {
    let val = val.unwrap_or(Value::none());
    val.update_list(|list| {
      list.extend(vals.iter().cloned());
      Ok(())
    })
  })
}
