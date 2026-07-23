use super::{EvalContext, FrameId, GLOBAL_FRAME, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  for name_val in args {
    if frame == GLOBAL_FRAME {
      continue;
    }

    let name = name_val.repr_str()?;

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
