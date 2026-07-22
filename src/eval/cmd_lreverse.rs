use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_var] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lreverse listVar".to_string(),
    ));
  };

  let mut list_var_val = eval_word(list_var, context, frame)?;
  let list_var_str = list_var_val.repr_str()?;
  let Some(mut list_val) = context.get_variable(frame, list_var_str).cloned() else {
    return Err(EvalError::UndefinedVariable(list_var_str.to_string()));
  };

  let mut list_list = list_val.repr_list()?.as_ref().clone();
  list_list.reverse();

  Ok(Value::from(list_list))
}
