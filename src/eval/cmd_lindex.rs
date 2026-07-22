use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [list_val, index] = words else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: lindex listVal index".to_string(),
    ));
  };

  let mut list_val_val = eval_word(list_val, context, frame)?;
  let list_list = list_val_val.repr_list()?;

  let mut index_val = eval_word(index, context, frame)?;
  let index_int = index_val.repr_int()?;

  match usize::try_from(index_int).ok().and_then(|i| list_list.get(i)) {
    None => Ok(Value::none()),
    Some(value) => Ok(value.clone()),
  }
}
