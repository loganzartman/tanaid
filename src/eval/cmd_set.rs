use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (name, maybe_value) = match args {
    [name, value] => (name, Some(value)),
    [name] => (name, None),
    [] => return Err(EvalError::Generic("missing variable name".to_string())),
    _ => {
      return Err(EvalError::Generic(
        "too many arguments; expects: set name ?value?".to_string(),
      ));
    }
  };

  if let Some(value) = maybe_value {
    context.set_variable(frame, name.repr_str()?, value.clone());
    Ok(value.clone())
  } else {
    Ok(
      context
        .get_variable(frame, name.repr_str()?)
        .ok_or_else(|| EvalError::UndefinedVariable(format!("{}", name.to_string())))?
        .clone(),
    )
  }
}
