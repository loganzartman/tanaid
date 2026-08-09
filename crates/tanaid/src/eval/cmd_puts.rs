use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let string = match args {
    [_, _, _] => todo!(),
    [_, _] => todo!(),
    [string] => string,
    [..] => {
      return Err(EvalError::Generic(
        "too many arguments; expects string".to_string(),
      ));
    }
  };

  context.stdout.as_ref()(format!("{}\n", string.repr_str()?).as_str()).map(|_| Value::none())
}
