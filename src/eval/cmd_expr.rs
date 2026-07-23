use super::{EvalContext, FrameId, cmd::EvalCmdResult, eval_expr};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let expr_src = if let [arg] = args {
    // optimization: no allocation for idiomatic single (braced) argument
    arg.repr_str()?
  } else {
    let values = args.iter_mut().map(|value| value.repr_str());
    &values.collect::<Result<Vec<&str>, _>>()?.join(" ")
  };

  let expr_parsed = context
    .parse_expr_caching(expr_src)
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (node, _) = expr_parsed.as_ref();

  eval_expr(&node, context, frame)
}
