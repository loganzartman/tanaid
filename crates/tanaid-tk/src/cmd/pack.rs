use crate::tk_context::{TkContext, Widget};
use tanaid::eval::EvalCmdResult;
use tanaid::eval::{EvalContext, FrameId};
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub(super) fn eval(
  args: &mut [Value],
  _ctx: &mut EvalContext,
  _frame: FrameId,
  tk: &TkContext,
) -> EvalCmdResult {
  match args {
    [widget_name] => {
      let widget_name_str = widget_name.repr_str()?;
      let widgets = tk.widgets.borrow();
      let Some(widget) = widgets.get(widget_name_str) else {
        return Err(EvalError::ArgumentError(format!(
          "pack: widget not found: {}",
          widget_name_str
        )));
      };

      match widget {
        Widget::Canvas(widget) => {
          tk.window_attributes.replace(Some(
            Window::default_attributes()
              .with_title("tanaid-tk")
              .with_inner_size(PhysicalSize::new(
                f64::from(widget.attrs.borrow().width.unwrap_or(256)),
                f64::from(widget.attrs.borrow().height.unwrap_or(256)),
              )),
          ));
        }
      }
    }
    _ => {
      return Err(EvalError::ArgumentError(
        "pack: expected exactly 1 argument".to_string(),
      ));
    }
  }
  Ok(Value::none())
}
