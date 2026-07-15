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
    "get" => eval_get(rest, context, frame),
    "has" => eval_has(rest, context, frame),
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
  let mut dict_val = words
    .get(0)
    .ok_or_else(|| {
      EvalError::ArgumentError(
        "dict get missing dictValue; expects dict get dictValue key".to_string(),
      )
    })
    .map(|w| eval_word(w, context, frame))??;

  let mut key_val = words
    .get(1)
    .ok_or_else(|| {
      EvalError::ArgumentError("dict get missing key; expects dict get dictValue key".to_string())
    })
    .map(|w| eval_word(w, context, frame))??;

  let dict = dict_val.repr_dict()?;
  let key_str = key_val.repr_str()?;
  let Some(result_val) = dict.get(key_str) else {
    return Err(EvalError::Generic(format!("dict missing key: {}", key_str)));
  };

  Ok(result_val.clone())
}

fn eval_has(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut dict_val = words
    .get(0)
    .ok_or_else(|| {
      EvalError::ArgumentError(
        "dict has missing dictValue; expects dict has dictValue key".to_string(),
      )
    })
    .map(|w| eval_word(w, context, frame))??;

  let mut key_val = words
    .get(1)
    .ok_or_else(|| {
      EvalError::ArgumentError("dict has missing key; expects dict has dictValue key".to_string())
    })
    .map(|w| eval_word(w, context, frame))??;

  let dict = dict_val.repr_dict()?;
  let key_str = key_val.repr_str()?;

  match dict.get(key_str) {
    Some(_) => Ok(Value::from(1)),
    None => Ok(Value::from(0)),
  }
}

fn eval_set(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut var_val = words
    .get(0)
    .ok_or_else(|| {
      EvalError::ArgumentError(
        "dict set missing dictVar; expects dict set dictVar key value".to_string(),
      )
    })
    .map(|w| eval_word(w, context, frame))??;

  let mut key_val = words
    .get(1)
    .ok_or_else(|| {
      EvalError::ArgumentError(
        "dict set missing key; expects dict set dictVar key value".to_string(),
      )
    })
    .map(|w| eval_word(w, context, frame))??;

  let val_val = words
    .get(2)
    .ok_or_else(|| {
      EvalError::ArgumentError(
        "dict set missing value; expects dict set dictVar key value".to_string(),
      )
    })
    .map(|w| eval_word(w, context, frame))??;

  let var_str = var_val.repr_str()?;

  let mut dict_val = context
    .get_variable(frame, var_str)
    .ok_or_else(|| EvalError::UndefinedVariable(format!("undefined variable: {}", var_str)))?
    .clone();

  let mut dict = dict_val.repr_dict()?.as_ref().clone();
  let key_str = key_val.repr_str()?;
  dict.insert(key_str.to_string(), val_val);
  let new_dict_val = Value::from(dict);
  context.set_variable(frame, var_str, new_dict_val.clone());

  Ok(new_dict_val)
}
