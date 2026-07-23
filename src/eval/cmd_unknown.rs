use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let [name, ..] = args else {
    return Err(EvalError::ArgumentError(
      "not enough arguments; expects: unknown commandName ?arg ...?".to_string(),
    ));
  };
  Err(EvalError::UndefinedCommand(name.repr_str()?.to_string()))
}
