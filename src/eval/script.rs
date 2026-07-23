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
  let mut words_evaled = command
    .words
    .iter()
    .map(|word| eval_word(word, context, frame))
    .collect::<Result<Vec<_>, _>>()?;

  let [name, args @ ..] = words_evaled.as_mut_slice() else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let name_str = name.repr_str()?;

  // user-defined proc
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
