use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::extract_target_frame_and_rest::extract_target_frame_and_rest;
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str =
  "wrong number of args; expected: upvar ?level? otherVar myVar ?otherVar myVar ...?";

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (target_frame, rest) = extract_target_frame_and_rest(args, frame, WRONG_ARGS_MSG)?;

  let mut it = rest.iter_mut();
  loop {
    let Some(other_var) = it.next() else {
      break;
    };
    let Some(my_var) = it.next() else {
      return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
    };

    context.ref_variable(
      frame,
      my_var.repr_str()?,
      target_frame,
      other_var.repr_str()?,
    );
  }

  Ok(Value::none())
}
