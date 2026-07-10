use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  match words.get(0).map(|w| eval_word(w, context, frame)) {
    Some(Ok(val)) => Err(EvalError::ReturnError(val)),
    Some(Err(e)) => Err(e),
    None => Err(EvalError::ReturnError(Value::none())),
  }?
}
