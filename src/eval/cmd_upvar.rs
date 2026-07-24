use super::{EvalContext, FrameId, cmd::EvalCmdResult, context::GLOBAL_FRAME};
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str =
  "wrong number of args; expected: upvar ?level? otherVar myVar ?otherVar myVar ...?";

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  if args.len() < 2 {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  }

  let (target_frame, rest) = if args.len() % 2 == 1 {
    let (first, rest) = args.split_first_mut().unwrap();
    (resolve_level(first, frame)?, rest)
  } else {
    let default = frame
      .checked_sub(1)
      .ok_or_else(|| EvalError::ArgumentError("invalid level: 1".to_string()))?;
    (default, &mut *args)
  };

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

fn resolve_level(arg: &mut Value, frame: FrameId) -> Result<usize, EvalError> {
  let level_str = arg.repr_str()?.to_string();
  let invalid = || EvalError::ArgumentError(format!("invalid level: {}", level_str));

  let resolved = if let Some(abs) = level_str.strip_prefix('#') {
    let i = Value::new(abs).repr_int().map_err(|_| invalid())?;
    usize::try_from(i)
      .ok()
      .and_then(|u| GLOBAL_FRAME.checked_add(u))
      .ok_or_else(invalid)?
  } else {
    let i = arg.repr_int().map_err(|_| invalid())?;
    usize::try_from(i)
      .ok()
      .and_then(|u| frame.checked_sub(u))
      .ok_or_else(invalid)?
  };

  Ok(resolved)
}
