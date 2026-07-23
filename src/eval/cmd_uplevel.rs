use super::{EvalContext, FrameId, GLOBAL_FRAME, cmd::EvalCmdResult};
use crate::eval::eval_script;
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str = "wrong number of args; expected: uplevel ?level? arg ?arg ...?";

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [level, rest @ ..] = args else {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  };
  if rest.is_empty() {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  }

  let mut body = String::new();
  for arg in rest {
    body.push_str(arg.repr_str()?);
  }

  let target_frame = match level.repr_str()? {
    absolute if absolute.starts_with('#') => {
      usize::try_from(Value::new(absolute.strip_prefix('#').unwrap()).repr_int()?)
        .map(|u| GLOBAL_FRAME + u)
        .ok()
    }
    _ => usize::try_from(level.repr_int()?)
      .ok()
      .and_then(|u| frame.checked_sub(u)),
  };
  let Some(target_frame) = target_frame else {
    return Err(EvalError::ArgumentError(format!(
      "invalid level: {}",
      level
    )));
  };

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
