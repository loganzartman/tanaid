use super::{EvalContext, FrameId, cmd::EvalCmdResult, context::GLOBAL_FRAME};
use crate::eval::eval_script;
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str = "wrong number of args; expected: uplevel ?level? arg ?arg ...?";

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (target_frame, rest) = extract_target_frame_and_rest(args, frame, WRONG_ARGS_MSG)?;

  let mut body = String::new();
  for arg in rest {
    body.push_str(arg.repr_str()?);
  }

  let script_result = match context.parse_script_caching(body.as_str()) {
    Ok(result) => result,
    Err(e) => {
      return Err(EvalError::ScriptParseError(format!(
        "failed to parse uplevel body: {}",
        e
      )));
    }
  };
  let (script, _) = script_result.as_ref();

  eval_script(script, context, target_frame)
}

fn extract_target_frame_and_rest<'a>(
  args: &'a mut [Value],
  frame: FrameId,
  wrong_args_msg: &str,
) -> Result<(usize, &'a mut [Value]), EvalError> {
  let Some(first) = args.first_mut() else {
    return Err(EvalError::ArgumentError(wrong_args_msg.to_string()));
  };

  enum Level {
    Abs(i64),
    Rel(i64),
  }

  let (level, used_first) = match first.repr_str()? {
    abs if abs.starts_with('#') => {
      if let Ok(val) = Value::new(abs.strip_prefix('#').unwrap()).repr_int() {
        (Level::Abs(val), true)
      } else {
        (Level::Rel(1), false)
      }
    }
    _ => {
      if let Ok(val) = first.repr_int() {
        (Level::Rel(val), true)
      } else {
        (Level::Rel(1), false)
      }
    }
  };

  let resolved_level = match level {
    Level::Abs(i) => usize::try_from(i)
      .ok()
      .and_then(|u| GLOBAL_FRAME.checked_add(u))
      .ok_or_else(|| EvalError::ArgumentError(format!("invalid level: {}", i))),
    Level::Rel(i) => usize::try_from(i)
      .ok()
      .and_then(|u| frame.checked_sub(u))
      .ok_or_else(|| EvalError::ArgumentError(format!("invalid level: {}", i))),
  }?;

  Ok((
    resolved_level,
    if used_first {
      &mut args[1..]
    } else {
      &mut args[0..]
    },
  ))
}
