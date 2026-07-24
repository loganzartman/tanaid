use super::{EvalContext, FrameId, cmd::EvalCmdResult, context::GLOBAL_FRAME};
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str =
  "wrong number of args; expected: upvar ?level? otherVar myVar ?otherVar myVar ...?";

enum Level {
  Abs(i64),
  Rel(i64),
}

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  if args.len() < 2 {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  }

  // upvar determines whether the first arg is a level by the arity of the arguments,
  // since the variable aliases must be in pairs.
  let (level, rest) = if args.len() % 2 == 1 {
    if let Some(abs_str) = args[0].repr_str()?.strip_prefix('#') {
      (Level::Abs(Value::new(abs_str).repr_int()?), &mut args[1..])
    } else {
      (Level::Rel(args[0].repr_int()?), &mut args[1..])
    }
  } else {
    (Level::Rel(1), args)
  };

  let target_frame = match level {
    Level::Abs(i) => usize::try_from(i)
      .ok()
      .and_then(|u| GLOBAL_FRAME.checked_add(u))
      .ok_or_else(|| EvalError::ArgumentError(format!("invalid level: {}", i))),
    Level::Rel(i) => usize::try_from(i)
      .ok()
      .and_then(|u| frame.checked_sub(u))
      .ok_or_else(|| EvalError::ArgumentError(format!("invalid level: {}", i))),
  }?;

  let mut it = rest.iter_mut();
  while let (Some(other_var), Some(my_var)) = (it.next(), it.next()) {
    context.ref_variable(
      frame,
      my_var.repr_str()?,
      target_frame,
      other_var.repr_str()?,
    );
  }

  Ok(Value::none())
}
