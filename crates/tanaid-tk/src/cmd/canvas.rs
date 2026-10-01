use crate::canvas::{CanvasAttributes, CanvasWidget};
use crate::cmd;
use crate::tk_context::TkContext;
use std::cell::RefCell;
use std::rc::Rc;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::{EvalContext, FrameId};
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  ctx: &mut EvalContext,
  _frame: FrameId,
  tk: &TkContext,
) -> EvalCmdResult {
  let (path_name, rest) = match args {
    [path_name, rest @ ..] => (path_name, rest),
    _ => {
      return Err(EvalError::ArgumentError(
        "canvas: missing path name".to_string(),
      ));
    }
  };

  let path_name_str = path_name.repr_str()?;

  if !path_name_str.starts_with(".") {
    return Err(EvalError::ArgumentError(
      "canvas: path name must start with '.'".to_string(),
    ));
  }

  let mut attrs = CanvasAttributes::new();

  let mut opts = rest.iter_mut();
  loop {
    let Some(option) = opts.next() else {
      break;
    };

    let option_str = option.repr_str()?;
    let option_str = option_str
      .strip_prefix('-')
      .ok_or(EvalError::ArgumentError(format!(
        "canvas: invalid option: {}",
        option_str
      )))?;

    match option_str {
      "width" => {
        let Some(value) = opts.next() else {
          return Err(EvalError::ArgumentError(
            "value for width missing".to_string(),
          ));
        };
        attrs.width = Some(
          u32::try_from(value.repr_int()?)
            .map_err(|e| EvalError::ArgumentError(format!("invalid width: {}", e)))?,
        );
      }
      "height" => {
        let Some(value) = opts.next() else {
          return Err(EvalError::ArgumentError(
            "value for width missing".to_string(),
          ));
        };
        attrs.height = Some(
          u32::try_from(value.repr_int()?)
            .map_err(|e| EvalError::ArgumentError(format!("invalid height: {}", e)))?,
        );
      }
      _ => {
        return Err(EvalError::ArgumentError(format!(
          "canvas: invalid option: {}",
          option_str
        )));
      }
    }
  }

  let widget = Rc::new(RefCell::new(CanvasWidget::new(attrs)));
  tk.widgets
    .borrow_mut()
    .insert(path_name_str.to_string(), widget.clone());

  {
    let tk = tk.clone();
    let path_name_string = path_name_str.to_string();

    // weak ref so removing the widget from the tk context can deallocate it
    let widget = Rc::downgrade(&widget);

    ctx.register_command(path_name_str, move |args, ctx, frame| {
      let (subcommand, rest) = match args {
        [subcommand, rest @ ..] => (subcommand.repr_str()?, rest),
        _ => {
          return Err(EvalError::ArgumentError(format!(
            "wrong number of args; should be: {} option ...",
            path_name_string
          )));
        }
      };

      let Some(widget) = widget.upgrade() else {
        return Err(EvalError::Generic(format!(
          "Widget no longer exists: {}",
          path_name_string
        )));
      };

      let widget = &mut widget.borrow_mut();
      match subcommand {
        "coords" => cmd::canvas_coords::eval(rest, ctx, frame, &tk, widget),
        "create" => cmd::canvas_create::eval(rest, ctx, frame, &tk, widget),
        "delete" => cmd::canvas_delete::eval(rest, ctx, frame, &tk, widget),
        _ => Err(EvalError::ArgumentError(format!(
          "canvas: invalid subcommand: {}",
          subcommand
        ))),
      }
    });
  }

  Ok(Value::from(path_name_str))
}
