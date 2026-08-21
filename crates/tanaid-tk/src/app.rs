use std::sync::Arc;
use tanaid::eval::EvalContext;
use tanaid::event_loop::EventLoop as TimerLoop;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::{Renderer, SoftwareRendering, Tk, TkError};

/// Shows the mapped widget in a window, and runs the interpreter's timers until
/// the window is closed. This is Tk's `vwait`/`wish` main loop: the window owns
/// the process from here on.
pub fn run(tk: Tk, context: &mut EvalContext) -> Result<(), TkError> {
  let event_loop = EventLoop::new().map_err(|e| TkError::Windowing(e.to_string()))?;

  let mut app = App {
    tk,
    context,
    timers: TimerLoop::new(),
    window: None,
    renderer: None,
    error: None,
  };
  let result = event_loop
    .run_app(&mut app)
    .map_err(|e| TkError::Windowing(e.to_string()));

  // the window is gone once its event loop returns
  app.tk.unmap_widgets();
  result?;

  match app.error {
    Some(err) => Err(err),
    None => Ok(()),
  }
}

struct App<'a> {
  tk: Tk,
  context: &'a mut EvalContext,
  timers: TimerLoop,
  window: Option<Arc<Window>>,
  renderer: Option<Renderer>,
  error: Option<TkError>,
}

impl ApplicationHandler for App<'_> {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_some() {
      return;
    }

    if let Err(err) = self.open_window(event_loop) {
      self.fail(event_loop, err);
    }
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
    match event {
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(size) => {
        if let Some(renderer) = &mut self.renderer {
          renderer.resize(size.width, size.height);
        }
        if let Some(window) = &self.window {
          window.request_redraw();
        }
      }
      WindowEvent::RedrawRequested => {
        if let Err(err) = self.render() {
          self.fail(event_loop, err);
        }
      }
      _ => {}
    }
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    let next_deadline = match self.timers.poll(self.context) {
      Ok(next_deadline) => next_deadline,
      Err(err) => return self.fail(event_loop, err.into()),
    };

    if self.tk.take_dirty()
      && let Some(window) = &self.window
    {
      window.request_redraw();
    }

    event_loop.set_control_flow(match next_deadline {
      Some(deadline) => ControlFlow::WaitUntil(deadline),
      None => ControlFlow::Wait,
    });
  }
}

impl App<'_> {
  fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), TkError> {
    let size = {
      let canvas = self
        .tk
        .mapped_canvas()
        .ok_or_else(|| TkError::Windowing("no widget is mapped".to_string()))?;
      // a canvas is sized in pixels, so it gets pixels
      PhysicalSize::new(canvas.width, canvas.height)
    };

    let window = Arc::new(
      event_loop
        .create_window(
          Window::default_attributes()
            .with_title("tanaid")
            .with_inner_size(size),
        )
        .map_err(|e| TkError::Windowing(e.to_string()))?,
    );

    self.renderer = Some(create_renderer(event_loop, &window)?);
    self.window = Some(window);

    self.render()
  }

  fn render(&mut self) -> Result<(), TkError> {
    let Some(renderer) = &mut self.renderer else {
      return Ok(());
    };
    let Some(canvas) = self.tk.mapped_canvas() else {
      return Ok(());
    };

    renderer.render(&canvas)
  }

  fn fail(&mut self, event_loop: &ActiveEventLoop, err: TkError) {
    self.error = Some(err);
    event_loop.exit();
  }
}

fn create_renderer(
  event_loop: &ActiveEventLoop,
  window: &Arc<Window>,
) -> Result<Renderer, TkError> {
  let size = window.inner_size();
  let mut last_error = None;

  for (backends, software_rendering) in backend_attempts() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends,
      // GLES on Wayland needs the compositor connection the window came from
      ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
        event_loop.owned_display_handle(),
      ))
    });

    let result = instance
      .create_surface(window.clone())
      .map_err(|e| TkError::Graphics(e.to_string()))
      .and_then(|surface| {
        pollster::block_on(Renderer::new(
          instance,
          surface,
          size.width,
          size.height,
          software_rendering,
        ))
      })
      .and_then(|mut renderer| renderer.probe_present().map(|()| renderer));

    match result {
      Ok(renderer) => return Ok(renderer),
      Err(err) => last_error = Some(err),
    }
  }

  Err(last_error.unwrap_or_else(|| TkError::Graphics("no graphics backend".to_string())))
}

/// What to try, in order: a hardware adapter on any backend, then a hardware
/// adapter on GL, then whatever is left. GL gets its own attempt because the
/// preferred backend can turn out to be a software rasterizer that cannot
/// present — WSL offers Vulkan through llvmpipe, but real GL through D3D12.
/// `WGPU_BACKEND` still restricts every attempt.
fn backend_attempts() -> Vec<(wgpu::Backends, SoftwareRendering)> {
  let preferred = wgpu::Backends::default().with_env();
  let gl = wgpu::Backends::GL.with_env();

  let mut attempts = vec![(preferred, SoftwareRendering::Rejected)];
  if gl != preferred {
    attempts.push((gl, SoftwareRendering::Rejected));
  }
  attempts.push((preferred, SoftwareRendering::Allowed));

  attempts
}
