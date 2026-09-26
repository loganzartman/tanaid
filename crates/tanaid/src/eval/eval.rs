use crate::eval::EvalContext;
use crate::eval::context::GLOBAL_FRAME;
use crate::eval::script::eval_returnable_script;
use crate::eval_error::EvalError;
use crate::parser::ScriptNode;
use crate::value::Value;

pub async fn eval(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  eval_returnable_script(script, context, GLOBAL_FRAME).await
}

pub fn eval_blocking(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  pollster::block_on(eval(script, context))
}
