use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_val] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lreverse listVal".to_string(),
    ));
  };

  let mut list_val_val = eval_word(list_val, context, frame)?;
  let mut list_list = list_val_val.repr_list()?.as_ref().clone();
  list_list.reverse();

  Ok(Value::from(list_list))
}
