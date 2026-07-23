use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_script;
use crate::eval::extract_target_frame_and_rest::extract_target_frame_and_rest;
use crate::eval_error::EvalError;
use crate::value::Value;

const WRONG_ARGS_MSG: &str = "wrong number of args; expected: {} ?level? arg ?arg ...?";

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
