use super::{EvalContext, FrameId, cmd::eval_builtin, eval_proc, eval_word};
use crate::eval_error::EvalError;
use crate::parser::{CommandNode, ScriptNode};
use crate::value::Value;

pub fn eval_returnable_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    match eval_command(&command, context, frame) {
      Ok(val) => result = val,
      Err(EvalError::ReturnError(val)) => {
        result = val;
        break;
      }
      err => return err,
    }
  }
  Ok(result)
}

pub fn eval_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    result = eval_command(&command, context, frame)?;
  }
  Ok(result)
}

pub fn eval_command(
  command: &CommandNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let [name, args @ ..] = command.words.as_slice() else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let mut name_value = eval_word(&name, context, frame)?;
  let name_str = name_value.repr_str()?;

  // user-defined proc
  // TODO: reference-count procs
  if let Some(proc) = context.get_proc(name_str) {
    return eval_proc(name_str, &proc, args, context, frame);
  }

  // builtin
  if let Some(result) = eval_builtin(name_str, args, context, frame) {
    result
  } else {
    Err(EvalError::UndefinedCommand(name_str.to_string()))
  }
}
