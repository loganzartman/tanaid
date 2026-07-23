use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (var, increment) = match args {
    [var, increment] => (var, Some(increment)),
    [var] => (var, None),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of arguments; expects: incr varName ?increment?".to_string(),
      ));
    }
  };

  let increment_val = increment.map(|val| val.repr_int()).unwrap_or(Ok(1))?;

  let var_str = var.repr_str()?;
  let mut var_val = match context.get_variable(frame, var_str).cloned() {
    Some(val) => val,
    None => Value::from(0),
  };

  let new_var_val = Value::from(var_val.repr_int()? + increment_val);
  context.set_variable(frame, var_str, new_var_val.clone());
  Ok(new_var_val)
}
