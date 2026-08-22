use crate::tk_context::TkContext;
use std::rc::Rc;
use std::time::Duration;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};

pub struct Tk {
  event_loop: winit::event_loop::EventLoop<()>,
  app: TkApp,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      event_loop: winit::event_loop::EventLoop::new().unwrap(),
      app: TkApp {
        context: Rc::new(TkContext::new()),
      },
    }
  }

  pub fn install(&mut self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    {
      let context = self.app.context.clone();
      ctx.register_command(
        "canvas",
        Rc::new(move |args, ctx, frame| context.canvas(args, ctx, frame)),
      );
    }
    {
      let context = self.app.context.clone();
      ctx.register_command(
        "pack",
        Rc::new(move |args, ctx, frame| context.pack(args, ctx, frame)),
      );
    }

    Ok(())
  }

  pub fn pump_app_events(&mut self) -> PumpStatus {
    self
      .event_loop
      .pump_app_events(Some(Duration::ZERO), &mut self.app)
  }
}

struct TkApp {
  context: Rc<TkContext>,
}

impl ApplicationHandler for TkApp {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.context.handle_resumed(event_loop);
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
