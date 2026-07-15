use crate::{
  eval::{EvalContext, FrameId, cmd::EvalCmdResult, eval_word},
  eval_error::EvalError,
  parser::WordNode,
  value::Value,
};

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut subcommand = words
    .get(0)
    .ok_or_else(|| EvalError::ArgumentError("string requires subcommand".to_string()))
    .map(|w| eval_word(w, context, frame))??;

  let subcommand_str = subcommand.repr_str()?;
  let rest = &words[1..];
  match subcommand_str {
    "index" => eval_index(rest, context, frame),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported string subcommand: {}",
      subcommand_str
    ))),
  }
}

fn eval_index(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [string_word, index_word] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: string index string charIndex".to_string(),
    ));
  };

  let mut string_val = eval_word(string_word, context, frame)?;
  let mut index_val = eval_word(index_word, context, frame)?;

  let index = usize::try_from(index_val.repr_int()?)
    .map_err(|_| EvalError::ArgumentError("string index cannot be negative".to_string()))?;
  let ch = string_val
    .repr_str()?
    .chars()
    .nth(index)
    .ok_or_else(|| EvalError::ArgumentError(format!("invalid index: {}", index)))?;

  Ok(Value::from(String::from(ch)))
}
