use crate::cmd::canvas::CanvasWidget;
use crate::tk_context::TkContext;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub(crate) fn eval(
  args: &mut [Value],
  _ctx: &mut EvalContext,
  _frame: FrameId,
  _tk: &TkContext,
  widget: &CanvasWidget,
) -> EvalCmdResult {
  let mut ids = vec![];
  for arg in args {
    ids.push(arg.repr_int()?);
  }

  if ids.is_empty() {
    return Err(EvalError::ArgumentError(
      "wrong number of args; should be: pathName delete ?tagOrId ...?".to_string(),
    ));
  }

  for id in ids {
    widget.items.borrow_mut().shift_remove(&id);
  }
  Ok(Value::none())
}
