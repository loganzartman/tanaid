use softbuffer::Surface;
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZero;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::{Window, WindowAttributes};

pub struct TkContext {
  widgets: RefCell<HashMap<String, Widget>>,
  window_attributes: RefCell<Option<WindowAttributes>>,
  surface: RefCell<Option<Surface<OwnedDisplayHandle, Rc<Window>>>>,
}

pub enum Widget {
  Canvas(CanvasAttributes),
}

pub struct CanvasAttributes {
  width: Option<u32>,
  height: Option<u32>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      widgets: RefCell::new(HashMap::new()),
      window_attributes: RefCell::new(None),
      surface: RefCell::new(None),
    }
  }

  pub(crate) fn canvas(
    &self,
    args: &mut [Value],
    ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
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

    self
      .widgets
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

  pub(crate) fn pack(
    &self,
    args: &mut [Value],
    _ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    match args {
      [widget_name] => {
        let widget_name_str = widget_name.repr_str()?;
        let widgets = self.widgets.borrow();
        let Some(widget) = widgets.get(widget_name_str) else {
          return Err(EvalError::ArgumentError(format!(
            "pack: widget not found: {}",
            widget_name_str
          )));
        };

        match widget {
          Widget::Canvas(attrs) => {
            self.window_attributes.replace(Some(
              Window::default_attributes()
                .with_title("tanaid-tk")
                .with_inner_size(LogicalSize::new(
                  f64::from(attrs.width.unwrap_or(256)),
                  f64::from(attrs.height.unwrap_or(256)),
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

  fn ensure_window(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if self.window_attributes.borrow().is_none() || self.surface.borrow().is_some() {
      return;
    }

    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();
    let window = Rc::new(
      event_loop
        .create_window(self.window_attributes.borrow().clone().unwrap())
        .unwrap(),
    );
    window.focus_window();
    let surface = Surface::new(&context, window).unwrap();
    self.surface.replace(Some(surface));
  }

  fn redraw(&self) {
    let mut surface = self.surface.borrow_mut();
    let Some(surface) = surface.as_mut() else {
      return;
    };

    let size = surface.window().inner_size();
    let Some(width) = NonZero::new(size.width) else {
      return;
    };
    let Some(height) = NonZero::new(size.height) else {
      return;
    };
    if surface.resize(width, height).is_err() {
      return;
    }
    let Ok(mut buffer) = surface.buffer_mut() else {
      return;
    };
    buffer.fill(0xFF808080);
    let _ = buffer.present();
  }

  pub fn handle_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.ensure_window(event_loop);
  }

  pub fn handle_about_to_wait(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.ensure_window(event_loop);
  }

  pub fn handle_window_event(
    &self,
    _event_loop: &winit::event_loop::ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
  ) {
    match event {
      WindowEvent::Resized(_) => {
        if let Some(surface) = self.surface.borrow().as_ref() {
          surface.window().request_redraw();
        }
      }
      WindowEvent::RedrawRequested => {
        self.redraw();
      }
      WindowEvent::CloseRequested => {
        self.window_attributes.replace(None);
        self.surface.replace(None);
      }
      _ => {}
    }
  }
}
