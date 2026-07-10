use super::{EvalContext, FrameId, Proc, cmd::EvalCmdResult, eval_word};
use crate::eval_error::EvalError;
use crate::parser::{self, WordNode};
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let (mut name_val, mut params_val, mut body_val) = match words {
    [name, params, body] => (
      eval_word(name, context, frame)?,
      eval_word(params, context, frame)?,
      eval_word(body, context, frame)?,
    ),
    [..] => {
      return Err(EvalError::ArgumentError(
        "wrong number of arguments; expects: proc name params body".to_string(),
      ));
    }
  };

  let name = name_val.repr_str()?;

  // args list items are not eval'ed; parse list and convert to literal strings
  let (params, "") = parser::parse_list(params_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc params must be a list: {}", e)))?
  else {
    return Err(EvalError::ArgumentError(
      "proc params must be a list: trailing input".to_string(),
    ));
  };

  let parsed = context
    .parse_script_caching(body_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc body must be a script: {}", e)))?;
  let (body, rest) = parsed.as_ref();

  if !rest.is_empty() {
    return Err(EvalError::ArgumentError(
      "proc body must be a script: trailing input".to_string(),
    ));
  };

  let body = body.clone();
  context.set_proc(name, Proc { params, body });
  Ok(Value::none())
}
