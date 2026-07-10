use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut args = words
    .iter()
    .map(|w| eval_word(w, context, frame))
    .peekable();

  let mut subcommand = args.next().ok_or_else(|| {
    EvalError::ArgumentError("not enough arguments; expects: info subcommand ...".to_string())
  })??;

  match subcommand.repr_str()? {
    "exists" => {
      let mut name = args.next().ok_or_else(|| {
        EvalError::ArgumentError("not enough arguments; expects: info exists varName".to_string())
      })??;
      if context.get_variable(frame, name.repr_str()?).is_some() {
        Ok(Value::from(1))
      } else {
        Ok(Value::from(0))
      }
    }
    unsupported => Err(EvalError::ArgumentError(format!(
      "unsupported info subcommand: {}",
      unsupported
    ))),
  }
}
