use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::{Dict, Value};

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut subcommand = words
    .get(0)
    .ok_or_else(|| EvalError::ArgumentError("dict requires subcommand".to_string()))
    .map(|w| eval_word(w, context, frame))??;

  let subcommand_str = subcommand.repr_str()?;
  let rest = &words[1..];
  match subcommand_str {
    "create" => eval_create(rest, context, frame),
    "exists" => eval_exists(rest, context, frame),
    "get" => eval_get(rest, context, frame),
    "set" => eval_set(rest, context, frame),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported dict subcommand: {}",
      subcommand_str
    ))),
  }
}

fn eval_create(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut dict = Dict::new();
  let mut it = words.iter();
  loop {
    let Some(k) = it.next() else { break };
    let Some(v) = it.next() else {
      return Err(EvalError::Generic(format!(
        "invalid dict; missing value for key {}",
        k
      )));
    };

    let mut k_val = eval_word(k, context, frame)?;
    let v_val = eval_word(v, context, frame)?;

    dict.insert(k_val.repr_str()?.to_string(), v_val);
  }

  Ok(Value::from(dict))
}

fn eval_get(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [val, keys @ ..] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict get dictValue key ?key ...?".to_string(),
    ));
  };

  let mut val_val = eval_word(val, context, frame)?;

  let mut keys_strs: Vec<String> = vec![];
  for key in keys {
    let mut key_val = eval_word(key, context, frame)?;
    keys_strs.push(String::from(key_val.repr_str()?));
  }

  for key in keys_strs {
    let child_dict = val_val.repr_dict()?;
    let Some(child_val) = child_dict.get(&key) else {
      return Err(EvalError::Generic(format!("dict missing key: {}", key)));
    };
    val_val = child_val.clone();
  }

  Ok(val_val)
}

fn eval_exists(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [val, keys @ ..] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict has dictValue key ?key ...?".to_string(),
    ));
  };

  if keys.is_empty() {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict has dictValue key ?key ...?".to_string(),
    ));
  }

  let mut val_val = eval_word(val, context, frame)?;

  let mut keys_strs: Vec<String> = vec![];
  for key in keys {
    let mut key_val = eval_word(key, context, frame)?;
    keys_strs.push(String::from(key_val.repr_str()?));
  }

  for key in keys_strs {
    let child_dict = val_val.repr_dict()?;
    let Some(child_val) = child_dict.get(&key) else {
      return Ok(Value::from(0));
    };
    val_val = child_val.clone();
  }

  Ok(Value::from(1))
}

fn eval_set(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [var, keys @ .., val] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict set dictVariable key ?key ...? value".to_string(),
    ));
  };

  let mut var_val = eval_word(var, context, frame)?;
  let var_str = var_val.repr_str()?;

  let mut keys_strs: Vec<String> = vec![];
  for key in keys {
    let mut key_val = eval_word(key, context, frame)?;
    keys_strs.push(String::from(key_val.repr_str()?));
  }

  let val_val = eval_word(val, context, frame)?;

  let dict_val = context
    .get_variable(frame, var_str)
    .ok_or_else(|| EvalError::UndefinedVariable(format!("undefined variable: {}", var_str)))?;

  let new_val = set_path(dict_val.clone(), keys_strs.as_slice(), &val_val)?;
  context.set_variable(frame, var_str, new_val.clone());

  Ok(new_val)
}

fn set_path(mut current: Value, path: &[String], value: &Value) -> EvalCmdResult {
  let Some((first, rest)) = path.split_first() else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict set dictVariable key ?key ...? value".to_string(),
    ));
  };

  let mut dict = current.repr_dict()?.as_ref().clone();

  if rest.is_empty() {
    dict.insert(first.clone(), value.clone());
  } else {
    let child = dict.get(first).cloned().unwrap_or(Value::from(Dict::new()));

    dict.insert(first.clone(), set_path(child, rest, value)?);
  }

  Ok(Value::from(dict))
}
