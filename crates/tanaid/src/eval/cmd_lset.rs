use std::mem::take;

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
  let indices_parsed = index_values(indices)?;

  // zero indices: replace variable value
  if indices_parsed.is_empty() {
    context.set_variable(frame, list_var_str, value.clone());
    return Ok(value.clone());
  }

  context.update_variable(frame, list_var_str, |list_val| {
    set_path(
      list_val.unwrap_or(Value::from(List::new())),
      indices_parsed,
      value.clone(),
    )
  })
}

fn set_path(list_val: Value, path: &[Value], new_val: Value) -> (Value, Result<(), EvalError>) {
  let (index, rest) = path.split_first().expect("path should be nonempty");
  list_val.update_list(|list| {
    let index = bounded_index(list, index)?;
    if rest.is_empty() {
      list[index] = new_val;
      Ok(())
    } else {
      let next = take(&mut list[index]);
      let (value, result) = set_path(next, rest, new_val);
      list[index] = value;
      result
    }
  })
}

fn bounded_index(list: &[Value], index: &Value) -> Result<usize, EvalError> {
  let index = index.repr_int()?;
  if index < 0 || index.saturating_cast::<usize>() >= list.len() {
    return Err(EvalError::ArgumentError(format!(
      "index out of range: {index}"
    )));
  }
  Ok(index as usize)
}

fn index_values(indices: &[Value]) -> Result<&[Value], EvalError> {
  Ok(if indices.len() == 1 {
    match indices[0].repr_int() {
      Ok(_) => indices,
      Err(_) => indices[0].repr_list()?.as_slice(),
    }
  } else {
    indices
  })
}
