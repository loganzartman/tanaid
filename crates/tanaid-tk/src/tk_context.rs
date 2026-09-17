use crate::cmd::canvas::CanvasWidget;
use crate::events::EventBindings;
use softbuffer::Surface;
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZero;
use std::rc::Rc;
use winit::event::WindowEvent;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::{Window, WindowAttributes};

#[derive(Clone)]
pub struct TkContext {
  pub(crate) widgets: Rc<RefCell<HashMap<String, Widget>>>,
  pub(crate) window_attributes: Rc<RefCell<Option<WindowAttributes>>>,
  pub(crate) surface: Rc<RefCell<Option<Surface<OwnedDisplayHandle, Rc<Window>>>>>,
  pub(crate) event_bindings: Rc<RefCell<EventBindings>>,
}

pub enum Widget {
  Canvas(CanvasWidget),
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      widgets: Rc::new(RefCell::new(HashMap::new())),
      window_attributes: Rc::new(RefCell::new(None)),
      surface: Rc::new(RefCell::new(None)),
      event_bindings: Rc::new(RefCell::new(EventBindings::new())),
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

    // TODO: Track packed widgets and layout instead of drawing every registered widget.
    for widget in self.widgets.borrow().values() {
      match widget {
        Widget::Canvas(widget) => widget.redraw(&mut buffer),
      }
    }

    let _ = buffer.present();
  }

  /// Whether a window is open (or is requested and pending creation).
  pub fn has_window(&self) -> bool {
    self.window_attributes.borrow().is_some() || self.surface.borrow().is_some()
  }

  pub fn handle_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.ensure_window(event_loop);
  }

  pub fn handle_about_to_wait(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.ensure_window(event_loop);
  }

  pub fn handle_window_event(
    &self,
    _window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
    tcl_event_loop: Rc<RefCell<tanaid::event_loop::EventLoop>>,
  ) {
    match event {
      WindowEvent::Resized(_) => {
        if let Some(surface) = self.surface.borrow().as_ref() {
          surface.window().request_redraw();
        }
      }
      WindowEvent::RedrawRequested => {
        self.redraw();
        if let Some(surface) = self.surface.borrow().as_ref() {
          surface.window().request_redraw();
        }
      }
      WindowEvent::CloseRequested => {
        self.window_attributes.replace(None);
        self.surface.replace(None);
      }
      WindowEvent::ModifiersChanged(mods) => {
        self.event_bindings.borrow_mut().handle_modifiers(mods)
      }
      WindowEvent::KeyboardInput { event, .. } => {
        self
          .event_bindings
          .borrow()
          .handle_key_event(".".to_string(), event, tcl_event_loop);
      }
      _ => {}
    }
  }
}
