use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_word;
use crate::parser::WordNode;
use crate::value::{List, Value};

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut list = List::new();
  for word in words {
    let value = eval_word(word, context, frame)?;
    list.push(value);
  }
  Ok(Value::from(list))
}
