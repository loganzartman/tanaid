use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_expr, eval_script, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [test, body] = words else {
    return Err(EvalError::Generic(
      "while requires two arguments: test and body".to_string(),
    ));
  };

  let mut expr_src = eval_word(test, context, frame)?;
  let expr_parsed = context
    .parse_expr_caching(expr_src.repr_str()?)
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (test_expr, _) = expr_parsed.as_ref();

  let mut body_src = eval_word(body, context, frame)?;
  let body_parsed = context
    .parse_script_caching(body_src.repr_str()?)
    .map_err(|e| EvalError::ScriptParseError(e.to_string()))?;
  let (body_script, _) = body_parsed.as_ref();

  while eval_expr(&test_expr, context, frame)?.repr_int()? != 0 {
    match eval_script(&body_script, context, frame) {
      Err(EvalError::BreakError) => break,
      Err(e) => return Err(e),
      Ok(_) => {}
    }
  }

  Ok(Value::none())
}
