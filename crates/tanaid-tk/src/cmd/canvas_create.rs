use crate::cmd::canvas::CanvasWidget;
use crate::cmd::canvas::{CanvasItem, Rect};
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
  let (item_type, rest) = match args {
    [item_type, rest @ ..] => (item_type.repr_str()?, rest),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of args; should be: pathName create itemType ?option ...?".to_string(),
      ));
    }
  };

  match item_type {
    "rectangle" => create_rect(rest, widget),
    _ => Err(EvalError::ArgumentError(format!(
      "invalid item type: {}",
      item_type
    ))),
  }
}

fn next_id(widget: &CanvasWidget) -> i64 {
  widget.items.borrow().last().map_or(0, |(id, _)| id + 1)
}

fn create_rect(opts: &mut [Value], widget: &CanvasWidget) -> EvalCmdResult {
  let (x1, y1, x2, y2, _rest) = match opts {
    [x1, y1, x2, y2, rest @ ..] => (
      x1.repr_float()?,
      y1.repr_float()?,
      x2.repr_float()?,
      y2.repr_float()?,
      rest,
    ),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of args; should be: pathName create rect x1 y1 x2 y2 ?option ...?"
          .to_string(),
      ));
    }
  };

  let id = next_id(widget);
  widget.items.borrow_mut().insert(
    id,
    CanvasItem::Rect(Rect::new().with_coords(x1, y1, x2, y2)),
  );

  Ok(Value::from(id))
}
