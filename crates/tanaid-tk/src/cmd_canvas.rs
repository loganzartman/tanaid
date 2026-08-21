use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

use crate::Tk;
use crate::canvas::{Canvas, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use crate::cmd_widget;
use crate::color::Color;

pub(crate) fn eval(
  tk: &Tk,
  args: &mut [Value],
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  let [path, options @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"canvas pathName ?option value ...?\"".to_string(),
    ));
  };

  let path = path.repr_str()?.to_string();
  if !path.starts_with('.') || path.len() < 2 {
    return Err(EvalError::ArgumentError(format!(
      "bad window path name \"{}\"",
      path
    )));
  }

  let mut canvas = Canvas::new(DEFAULT_WIDTH, DEFAULT_HEIGHT, Color::rgb(0xffffff));
  configure(&mut canvas, options)?;

  {
    let mut state = tk.state.borrow_mut();
    if state.canvases.contains_key(&path) {
      return Err(EvalError::ArgumentError(format!(
        "window name \"{}\" already exists",
        path
      )));
    }
    state.canvases.insert(path.clone(), canvas);
    state.dirty = true;
  }

  // the widget command, e.g. `.c create rectangle ...`
  let widget_tk = tk.clone();
  let widget_path = path.clone();
  context.register_command(
    path.as_str(),
    Rc::new(move |args, _context, _frame| cmd_widget::eval(&widget_tk, &widget_path, args)),
  );

  Ok(Value::from(path))
}

fn configure(canvas: &mut Canvas, options: &mut [Value]) -> Result<(), EvalError> {
  for option in options.chunks_mut(2) {
    let [name, value] = option else {
      return Err(EvalError::ArgumentError(format!(
        "value for \"{}\" missing",
        option[0].repr_str()?
      )));
    };

    let name = name.repr_str()?.to_string();
    match name.as_str() {
      "-width" => canvas.width = dimension(value)?,
      "-height" => canvas.height = dimension(value)?,
      "-background" | "-bg" => canvas.background = color(value)?,
      _ => {
        return Err(EvalError::ArgumentError(format!(
          "unsupported canvas option: {}",
          name
        )));
      }
    }
  }

  Ok(())
}

fn dimension(value: &mut Value) -> Result<u32, EvalError> {
  u32::try_from(value.repr_int()?)
    .map_err(|_| EvalError::ArgumentError("dimension must not be negative".to_string()))
}

fn color(value: &mut Value) -> Result<Color, EvalError> {
  let spec = value.repr_str()?;
  Color::parse(spec).ok_or_else(|| EvalError::ArgumentError(format!("unknown color: {}", spec)))
}
