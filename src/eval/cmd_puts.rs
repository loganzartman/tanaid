use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [mut string] = match words {
    [_, _, _] => todo!(),
    [_, _] => todo!(),
    [string] => [eval_word(string, context, frame)?],
    [..] => {
      return Err(EvalError::Generic(
        "too many arguments; expects string".to_string(),
      ));
    }
  };

  println!("{}", string.repr_str()?);
  Ok(Value::none())
}
