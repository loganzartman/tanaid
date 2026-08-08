use crate::{
  eval::{EvalContext, FrameId, cmd::EvalCmdResult},
  eval_error::EvalError,
  value::Value,
};

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let mut subcommand = args
    .get(0)
    .ok_or_else(|| EvalError::ArgumentError("dict requires subcommand".to_string()))?
    .clone();

  let subcommand_str = subcommand.repr_str()?;
  let rest = &mut args[1..];
  match subcommand_str {
    "index" => eval_index(rest),
    "length" => eval_length(rest),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported string subcommand: {}",
      subcommand_str
    ))),
  }
}

fn eval_index(args: &mut [Value]) -> EvalCmdResult {
  let [string_val, index_val] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: string index string charIndex".to_string(),
    ));
  };

  let index = usize::try_from(index_val.repr_int()?)
    .map_err(|_| EvalError::ArgumentError("string index cannot be negative".to_string()))?;
  let ch = string_val
    .repr_str()?
    .chars()
    .nth(index)
    .ok_or_else(|| EvalError::ArgumentError(format!("invalid index: {}", index)))?;

  Ok(Value::from(String::from(ch)))
}

fn eval_length(args: &mut [Value]) -> EvalCmdResult {
  let [string_val] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: string length string".to_string(),
    ));
  };

  Ok(Value::from(string_val.repr_str()?.chars().count() as i64))
}
