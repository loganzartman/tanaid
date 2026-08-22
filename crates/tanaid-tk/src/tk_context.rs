use std::cell::RefCell;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use winit::event::WindowEvent;
use winit::window::Window;

pub struct TkContext {
  window: RefCell<Option<winit::window::Window>>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      window: RefCell::new(None),
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
    Ok(Value::none())
  }

  pub(crate) fn handle_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.window.replace(Some(
      event_loop
        .create_window(Window::default_attributes())
        .unwrap(),
    ));
  }

  pub(crate) fn handle_window_event(
    &self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
  ) {
    match event {
      WindowEvent::CloseRequested => {
        println!("The close button was pressed; stopping");
        event_loop.exit();
      }
      _ => {}
    }
  }
}
