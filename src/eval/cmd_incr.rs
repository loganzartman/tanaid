use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (incr_var, increment) = match words {
    [incr_var, increment] => (incr_var, Some(increment)),
    [incr_var] => (incr_var, None),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of arguments; expects: incr varName ?increment?".to_string(),
      ));
    }
  };

  let mut incr_var_val = eval_word(incr_var, context, frame)?;
  let mut increment_val = match increment {
    Some(increment) => eval_word(increment, context, frame)?,
    None => Value::from(1),
  };

  let incr_var_str = incr_var_val.repr_str()?;
  let mut incr_val = match context.get_variable(frame, incr_var_str).cloned() {
    Some(val) => val,
    None => Value::from(0),
  };

  let new_var_val = Value::from(incr_val.repr_int()? + increment_val.repr_int()?);
  context.set_variable(frame, incr_var_str, new_var_val.clone());
  Ok(new_var_val)
}
