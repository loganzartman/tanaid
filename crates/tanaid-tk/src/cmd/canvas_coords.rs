use crate::canvas::CanvasWidget;
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
  widget: &mut CanvasWidget,
) -> EvalCmdResult {
  let (id, rest) = match args {
    [id, rest @ ..] => (id.repr_int()?, rest),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of args; should be: pathName coords id ?option ...?".to_string(),
      ));
    }
  };

  match rest {
    [] => {
      let Some(item) = widget.items.get(&id) else {
        return Ok(Value::none());
      };
      Ok(Value::from(item.get_coords()))
    }
    coords => {
      let Some(item) = widget.items.get_mut(&id) else {
        return Ok(Value::none());
      };

      let coords = coords
        .iter_mut()
        .map(|c| c.repr_float())
        .collect::<Result<Vec<f64>, _>>()?;
      item.set_coords(&coords)?;

      Ok(Value::none())
    }
  }
}
