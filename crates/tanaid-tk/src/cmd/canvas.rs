use super::EvalCmdResult;
use crate::tk_context::{CanvasAttributes, TkContext, Widget};
use std::rc::Rc;
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

  let mut attrs = CanvasAttributes {
    width: None,
    height: None,
  };

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

  tk.widgets
    .borrow_mut()
    .insert(path_name_str.to_string(), Widget::Canvas(attrs));

  ctx.register_command(
    path_name_str,
    Rc::new(move |args, ctx, _frame| {
      ctx.write_stdout(format!("canvas: {:?}", args).as_str())?;
      Ok(Value::none())
    }),
  );

  Ok(Value::from(path_name_str))
}
