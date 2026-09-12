use super::{EvalContext, FrameId, eval_proc, eval_word};
use crate::eval_error::EvalError;
use crate::parser::{CommandNode, ScriptNode};
use crate::value::Value;

pub async fn eval_returnable_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    match Box::pin(eval_command(&command, context, frame)).await {
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

pub async fn eval_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    result = Box::pin(eval_command(&command, context, frame)).await?;
  }
  Ok(result)
}

/// Evaluate a command and return the result.
/// By convention, Box::pin before awaiting eval_command to break cyclical recursive await.
pub async fn eval_command(
  command: &CommandNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut words_evaled = Vec::with_capacity(command.words.len());
  for word in &command.words {
    words_evaled.push(eval_word(word, context, frame).await?);
  }
  let name_and_args = words_evaled.as_mut_slice();

  let [name, args @ ..] = name_and_args else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let name_str = name.repr_str()?;

  // user-defined proc
  if let Some(proc) = context.get_proc(name_str) {
    return eval_proc(name_str, &proc, args, context, frame).await;
  }

  // command
  if let Some(handler) = context.get_command(name_str) {
    return handler(args, context, frame).await;
  }

  // user-defined unknown handler
  if let Some(proc) = context.get_proc("unknown") {
    return eval_proc("unknown", &proc, name_and_args, context, frame).await;
  }

  // builtin unknown handler
  if let Some(handler) = context.get_command("unknown") {
    return handler(name_and_args, context, frame).await;
  }

  unreachable!("missing builtin handler for unknown command");
}
