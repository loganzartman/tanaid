use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::{List, Value};

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_var, vals @ ..] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lappend listVar ?value value value ...?".to_string(),
    ));
  };

  let mut list_var_val = eval_word(list_var, context, frame)?;
  let mut val_vals: Vec<Value> = vec![];
  for val in vals {
    val_vals.push(eval_word(val, context, frame)?);
  }

  let list_var_str = list_var_val.repr_str()?;
  let mut list_val = match context.get_variable(frame, list_var_str).cloned() {
    Some(list) => list,
    None => Value::from(List::new()),
  };

  let mut list_list = list_val.repr_list()?.as_ref().clone();
  for val in val_vals {
    list_list.push(val);
  }

  let new_var_val = Value::from(list_list);
  context.set_variable(frame, list_var_str, new_var_val.clone());
  Ok(new_var_val)
}
