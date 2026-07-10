use super::{EvalContext, FrameId, GLOBAL_FRAME, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  for word in words {
    let mut name_node = eval_word(word, context, frame)?;

    if frame == GLOBAL_FRAME {
      continue;
    }

    let name = name_node.repr_str()?;

    if context.get_variable(frame, name).is_some() {
      return Err(EvalError::ArgumentError(format!(
        "local variable {} would be overwritten by global",
        name
      )));
    }

    context.ref_variable(frame, name, GLOBAL_FRAME, name);
  }

  Ok(Value::none())
}
