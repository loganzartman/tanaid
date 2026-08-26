use crate::cmd;
use crate::tk_context::{TkContext, Widget};
use indexmap::IndexMap;
use softbuffer::Buffer;
use std::cell::RefCell;
use std::rc::Rc;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::{EvalContext, FrameId};
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

#[derive(Clone)]
pub struct CanvasWidget {
  pub attrs: Rc<RefCell<CanvasAttributes>>,
  pub items: Rc<RefCell<IndexMap<i64, CanvasItem>>>,
}

pub struct CanvasAttributes {
  pub width: Option<u32>,
  pub height: Option<u32>,
}

impl CanvasAttributes {
  pub fn new() -> Self {
    Self {
      width: None,
      height: None,
    }
  }
}

pub enum CanvasItem {
  Rect(Rect),
}

impl CanvasItem {
  pub fn get_coords(&self) -> Vec<f64> {
    match self {
      CanvasItem::Rect(rect) => rect.get_coords(),
    }
  }

  pub fn redraw(&self, buffer: &mut Buffer<'_, OwnedDisplayHandle, Rc<Window>>) {
    match self {
      CanvasItem::Rect(rect) => rect.redraw(buffer),
    }
  }
}

pub struct Rect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

impl CanvasWidget {
  pub fn new(attrs: CanvasAttributes) -> Self {
    Self {
      attrs: Rc::new(RefCell::new(attrs)),
      items: Rc::new(RefCell::new(IndexMap::new())),
    }
  }
}

impl CanvasWidget {
  pub fn redraw(&self, buffer: &mut Buffer<'_, OwnedDisplayHandle, Rc<Window>>) {
    buffer.fill(0xFF000000);
    for item in self.items.borrow().values() {
      item.redraw(buffer);
    }
  }
}

impl Rect {
  pub fn new() -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      width: 0.0,
      height: 0.0,
    }
  }

  pub fn with_coords(mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
    self.set_coords(x1, y1, x2, y2);
    self
  }

  pub fn set_coords(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
    self.x = x1.min(x2);
    self.y = y1.min(y2);
    self.width = x2.max(x1) - self.x;
    self.height = y2.max(y1) - self.y;
  }

  pub fn get_coords(&self) -> Vec<f64> {
    vec![self.x, self.y, self.x + self.width, self.y + self.height]
  }

  pub fn redraw(&self, buffer: &mut Buffer<'_, OwnedDisplayHandle, Rc<Window>>) {
    for x in (self.x.round() as i64)..(self.x + self.width).round() as i64 {
      for y in (self.y.round() as i64)..(self.y + self.height).round() as i64 {
        if x < 0 || x >= buffer.width().get() as i64 || y < 0 || y >= buffer.height().get() as i64 {
          continue;
        }

        let i = y * buffer.width().get() as i64 + x;
        buffer[i as usize] = 0xFFFFFFFF;
      }
    }
  }
}

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

  let widget = CanvasWidget::new(attrs);
  tk.widgets
    .borrow_mut()
    .insert(path_name_str.to_string(), Widget::Canvas(widget.clone()));

  {
    let tk = tk.clone();
    let widget = widget.clone();
    let path_name_string = path_name_str.to_string();
    ctx.register_command(
      path_name_str,
      Rc::new(move |args, ctx, frame| {
        let (subcommand, rest) = match args {
          [subcommand, rest @ ..] => (subcommand.repr_str()?, rest),
          _ => {
            return Err(EvalError::ArgumentError(format!(
              "wrong number of args; should be: {} option ...",
              path_name_string
            )));
          }
        };

        match subcommand {
          "coords" => cmd::canvas_coords::eval(rest, ctx, frame, &tk, &widget),
          "create" => cmd::canvas_create::eval(rest, ctx, frame, &tk, &widget),
          _ => Err(EvalError::ArgumentError(format!(
            "canvas: invalid subcommand: {}",
            subcommand
          ))),
        }
      }),
    );
  }

  Ok(Value::from(path_name_str))
}
