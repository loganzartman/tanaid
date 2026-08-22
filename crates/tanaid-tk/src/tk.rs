use crate::tk_context::TkContext;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;

pub struct Tk {
  context: Rc<TkContext>,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      context: Rc::new(TkContext::new()),
    }
  }

  pub fn install(&mut self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    {
      let context = self.context.clone();
      ctx.register_command(
        "canvas",
        Rc::new(move |args, ctx, frame| context.canvas(args, ctx, frame)),
      );
    }
    {
      let context = self.context.clone();
      ctx.register_command(
        "pack",
        Rc::new(move |args, ctx, frame| context.pack(args, ctx, frame)),
      );
    }

    Ok(())
  }
}

/// Dispatches winit events to a TkContext
pub struct TkApp {
  context: Rc<TkContext>,
}

impl TkApp {
  pub fn new(context: Rc<TkContext>) -> Self {
    Self { context }
  }
}

impl ApplicationHandler for TkApp {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.context.handle_resumed(event_loop);
  }

  fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.context.handle_about_to_wait(event_loop);
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    self
      .context
      .handle_window_event(event_loop, window_id, event);
  }
}
