use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::{Dict, Value};

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut subcommand = args
    .get(0)
    .ok_or_else(|| EvalError::ArgumentError("dict requires subcommand".to_string()))?
    .clone();

  let subcommand_str = subcommand.repr_str()?;
  let rest = &mut args[1..];
  match subcommand_str {
    "create" => eval_create(rest),
    "exists" => eval_exists(rest),
    "get" => eval_get(rest),
    "set" => eval_set(rest, context, frame),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported dict subcommand: {}",
      subcommand_str
    ))),
  }
}

fn eval_create(args: &mut [Value]) -> EvalCmdResult {
  let mut dict = Dict::new();
  let mut it = args.iter_mut();
  loop {
    let Some(k) = it.next() else { break };
    let Some(v) = it.next() else {
      return Err(EvalError::Generic(format!(
        "invalid dict; missing value for key {}",
        k
      )));
    };

    dict.insert(k.repr_str()?.to_string(), v.clone());
  }

  Ok(Value::from(dict))
}

fn eval_get(args: &mut [Value]) -> EvalCmdResult {
  let [val, keys @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict get dictValue key ?key ...?".to_string(),
    ));
  };

  let mut current = val.clone();
  for key in keys {
    let dict = current.repr_dict()?;
    let Some(child_val) = dict.get(key.repr_str()?) else {
      return Err(EvalError::Generic(format!("dict missing key: {}", key)));
    };
    current = child_val.clone();
  }

  Ok(current)
}

fn eval_exists(args: &mut [Value]) -> EvalCmdResult {
  let [val, keys @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict has dictValue key ?key ...?".to_string(),
    ));
  };

  if keys.is_empty() {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict has dictValue key ?key ...?".to_string(),
    ));
  }

  let mut current = val.clone();
  for key in keys {
    let dict = current.repr_dict()?;
    let Some(child_val) = dict.get(key.repr_str()?) else {
      return Ok(Value::from(0));
    };
    current = child_val.clone();
  }

  Ok(Value::from(1))
}

fn eval_set(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [var, keys @ .., val] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments, expects: dict set dictVariable key ?key ...? value".to_string(),
    ));
  };

  let mut keys_strs: Vec<String> = vec![];
  for key in keys {
    keys_strs.push(String::from(key.repr_str()?));
  }

  let dict_val = context
    .get_variable(frame, var.repr_str()?)
    .ok_or_else(|| EvalError::UndefinedVariable(format!("undefined variable: {}", var)))?;

  let new_val = set_path(dict_val.clone(), keys_strs.as_slice(), val)?;
  context.set_variable(frame, var.repr_str()?, new_val.clone());

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
