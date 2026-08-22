use softbuffer::Surface;
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZero;
use std::rc::Rc;
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::{Window, WindowAttributes};

pub struct TkContext {
  pub(crate) widgets: RefCell<HashMap<String, Widget>>,
  pub(crate) window_attributes: RefCell<Option<WindowAttributes>>,
  pub(crate) surface: RefCell<Option<Surface<OwnedDisplayHandle, Rc<Window>>>>,
}

pub enum Widget {
  Canvas(CanvasAttributes),
}

pub struct CanvasAttributes {
  pub(crate) width: Option<u32>,
  pub(crate) height: Option<u32>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      widgets: RefCell::new(HashMap::new()),
      window_attributes: RefCell::new(None),
      surface: RefCell::new(None),
    }
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
