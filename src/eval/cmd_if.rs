use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_expr, eval_script, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::Value;

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let mut args = words
    .iter()
    .map(|w| eval_word(w, context, frame))
    .peekable();

  let mut cond_body: Vec<(Value, Value)> = vec![];

  loop {
    // require `elseif` to start 2nd condition onward
    if !cond_body.is_empty() {
      if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "elseif") {
        args.next();
      } else {
        break;
      }
    }

    // required condition
    let cond = args
      .next()
      .ok_or_else(|| EvalError::ArgumentError("expected condition".to_string()))??;

    // optional "then"
    if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "then") {
      args.next();
    }

    // required body
    let body = args
      .next()
      .ok_or_else(|| EvalError::ArgumentError("expected condition body".to_string()))??;

    cond_body.push((cond, body));
  }

  // optional "else"
  if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "else") {
    args.next();
  }

  // optional else body
  if let Some(else_body) = args.next() {
    cond_body.push((Value::from(1), else_body?));
  }

  for (cond, body) in &mut cond_body {
    let cond_parse_result = context
      .parse_expr_caching(cond.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if condition: {}", e)))?;
    let (cond_parsed, _) = cond_parse_result.as_ref();

    let body_parse_result = context
      .parse_script_caching(body.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if body: {}", e)))?;
    let (body_parsed, _) = body_parse_result.as_ref();

    if eval_expr(&cond_parsed, context, frame)?.repr_int()? != 0 {
      return eval_script(&body_parsed, context, frame);
    }
  }

  Ok(Value::none())
}
