use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (name, maybe_value) = match words {
    [name, value] => (name, Some(value)),
    [name] => (name, None),
    [] => return Err(EvalError::Generic("missing variable name".to_string())),
    _ => {
      return Err(EvalError::Generic(
        "too many arguments; expects: set name ?value?".to_string(),
      ));
    }
  };

  let mut name = eval_word(&name, context, frame)?;
  if let Some(value) = maybe_value {
    let value = eval_word(&value, context, frame)?;
    context.set_variable(frame, name.repr_str()?, value.clone());
    Ok(value)
  } else {
    Ok(
      context
        .get_variable(frame, name.repr_str()?)
        .ok_or_else(|| EvalError::UndefinedVariable(format!("{}", name.to_string())))?
        .clone(),
    )
  }
}
