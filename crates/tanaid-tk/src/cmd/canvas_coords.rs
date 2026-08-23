use crate::cmd::canvas::{CanvasItem, CanvasWidget};
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
      let items = widget.items.borrow();
      let Some(item) = items.get(&id) else {
        return Ok(Value::none());
      };
      Ok(Value::from(item.get_coords()))
    }
    coords => {
      let mut items = widget.items.borrow_mut();
      let Some(item) = items.get_mut(&id) else {
        return Ok(Value::none());
      };
      match item {
        CanvasItem::Rect(rect) => match coords {
          [x1, y1, x2, y2] => {
            rect.set_coords(
              x1.repr_float()?,
              y1.repr_float()?,
              x2.repr_float()?,
              y2.repr_float()?,
            );
          }
          _ => {
            return Err(EvalError::ArgumentError(
              "wrong number of args; should be: pathName coords id x1 y1 x2 y2".to_string(),
            ));
          }
        },
      }

      Ok(Value::none())
    }
  }
}
