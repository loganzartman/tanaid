use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_expr, eval_script};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [test, body] = args else {
    return Err(EvalError::Generic(
      "while requires two arguments: test and body".to_string(),
    ));
  };

  let test_parsed = context
    .parse_expr_caching(test.repr_str()?)
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (test_expr, _) = test_parsed.as_ref();

  let body_parsed = context
    .parse_script_caching(body.repr_str()?)
    .map_err(|e| EvalError::ScriptParseError(e.to_string()))?;
  let (body_script, _) = body_parsed.as_ref();

  while eval_expr(&test_expr, context, frame)?.repr_int()? != 0 {
    match eval_script(&body_script, context, frame) {
      Err(EvalError::BreakError) => break,
      Err(EvalError::ContinueError) => {}
      Err(e) => return Err(e),
      Ok(_) => {}
    }
  }

  Ok(Value::none())
}
