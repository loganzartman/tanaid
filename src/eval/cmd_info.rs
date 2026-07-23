use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [subcommand, rest @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "not enough arguments; expects: info subcommand ...".to_string(),
    ));
  };

  match subcommand.repr_str()? {
    "exists" => {
      let [name] = rest else {
        return Err(EvalError::ArgumentError(
          "not enough arguments; expects: info exists varName".to_string(),
        ));
      };
      if context.get_variable(frame, name.repr_str()?).is_some() {
        Ok(Value::from(1))
      } else {
        Ok(Value::from(0))
      }
    }
    unsupported => Err(EvalError::ArgumentError(format!(
      "unsupported info subcommand: {}",
      unsupported
    ))),
  }
}
