use crate::eval::EvalContext;
use crate::eval::context::GLOBAL_FRAME;
use crate::eval::script::eval_returnable_script;
use crate::eval_error::EvalError;
use crate::parser::ScriptNode;
use crate::value::Value;

/// Evaluate a script. You probably don't want to use this directly.
///
/// To just run a script, use [crate::run_blocking::run_blocking].
///
/// To integrate with an event loop, use [crate::interpreter::Interpreter].
pub async fn eval(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  eval_returnable_script(script, context, GLOBAL_FRAME).await
}
