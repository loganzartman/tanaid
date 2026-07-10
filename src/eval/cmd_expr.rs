use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_expr, eval_word};
use crate::eval_error::EvalError;
use crate::parser::{WordNode, WordPart};

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let expr_src = if let [word] = words
    && let [WordPart::BracedLiteral(braced_src)] = word.parts.as_slice()
  {
    // optimization: no allocation for idiomatic single braced argument
    braced_src
  } else {
    let values = words
      .iter()
      .map(|word| eval_word(&word, context, frame).map(|value| value.to_string()));
    &values.collect::<Result<Vec<String>, _>>()?.join(" ")
  };

  let expr_parsed = context
    .parse_expr_caching(expr_src.as_str())
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (node, _) = expr_parsed.as_ref();

  eval_expr(&node, context, frame)
}
