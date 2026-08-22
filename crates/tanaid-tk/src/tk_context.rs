use softbuffer::Surface;
use std::cell::{Cell, RefCell};
use std::num::NonZero;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

pub struct TkContext {
  show_window: Cell<bool>,
  surface: RefCell<Option<Surface<OwnedDisplayHandle, Rc<Window>>>>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      show_window: Cell::new(false),
      surface: RefCell::new(None),
    }
  }

  pub(crate) fn canvas(
    &self,
    args: &mut [Value],
    ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    let path_name = match args {
      [path_name, _rest @ ..] => path_name,
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
    _args: &mut [Value],
    _ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    self.show_window.set(true);
    Ok(Value::none())
  }

  fn ensure_window(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if !self.show_window.get() || self.surface.borrow().is_some() {
      return;
    }

    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();
    let window = Rc::new(
      event_loop
        .create_window(
          Window::default_attributes()
            .with_title("tanaid-tk")
            .with_inner_size(PhysicalSize::new(256, 256)),
        )
        .unwrap(),
    );
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
    event_loop: &winit::event_loop::ActiveEventLoop,
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
        println!("The close button was pressed; stopping");
        event_loop.exit();
      }
      _ => {}
    }
  }
}
