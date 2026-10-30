use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::{List, Value};

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_var, indices @ .., value] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lset listVar ?index ...? newValue".to_string(),
    ));
  };

  let list_var_str = list_var.repr_str();
  let list_val = match context.get_variable(frame, list_var_str).cloned() {
    Some(list) => list,
    None => Value::from(List::new()),
  };

  let indices_concat = indices
    .iter()
    .map(|index| index.repr_str().to_string())
    .collect::<Vec<String>>()
    .join(" ");

  let path = Value::from(indices_concat)
    .repr_list()?
    .clone()
    .iter()
    .map(|index| index.repr_int())
    .collect::<Result<Vec<i64>, EvalError>>()?;

  // zero indices: replace variable value
  if path.is_empty() {
    context.set_variable(frame, list_var_str, value.clone());
    return Ok(value.clone());
  }

  let new_val = set_path(list_val, path.as_slice(), value)?;
  context.set_variable(frame, list_var_str, new_val.clone());
  Ok(new_val)
}

fn set_path(current: Value, path: &[i64], value: &Value) -> EvalCmdResult {
  let Some((index, rest)) = path.split_first() else {
    return Ok(value.clone());
  };

  let mut current_list = current.repr_list()?.clone();

  if *index < 0 || *index >= current_list.len() as i64 {
    return Err(EvalError::ArgumentError(format!(
      "index out of range: {}",
      *index
    )));
  }

  current_list[*index as usize] = set_path(current_list[*index as usize].clone(), rest, value)?;

  Ok(Value::from(current_list))
}
