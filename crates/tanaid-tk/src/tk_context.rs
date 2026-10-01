use crate::events::EventBindings;
use crate::tk_renderer::TkRenderer;
use crate::widget::Widget;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::Affine;
use winit::event::WindowEvent;
use winit::window::{Window, WindowAttributes};

#[derive(Clone)]
pub struct TkContext {
  pub(crate) widgets: Rc<RefCell<HashMap<String, Rc<RefCell<dyn Widget>>>>>,
  pub(crate) window_attributes: Rc<RefCell<Option<WindowAttributes>>>,
  pub(crate) renderer: Rc<RefCell<Option<TkRenderer>>>,
  pub(crate) window: Rc<RefCell<Option<Arc<Window>>>>,
  pub(crate) event_bindings: Rc<RefCell<EventBindings>>,
  scene: Rc<RefCell<Scene>>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      widgets: Rc::new(RefCell::new(HashMap::new())),
      window_attributes: Rc::new(RefCell::new(None)),
      renderer: Rc::new(RefCell::new(None)),
      window: Rc::new(RefCell::new(None)),
      event_bindings: Rc::new(RefCell::new(EventBindings::new())),
      scene: Rc::new(RefCell::new(Scene::new())),
    }
  }

  fn ensure_window(
    &self,
    event_loop: &winit::event_loop::ActiveEventLoop,
  ) -> Result<(), EvalError> {
    if self.renderer.borrow().is_some() {
      return Ok(());
    }

    let Some(window_attributes) = self.window_attributes.borrow().clone() else {
      return Ok(());
    };

    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
    window.focus_window();
    let size = window.inner_size();

    let renderer = match pollster::block_on(TkRenderer::new(
      Arc::clone(&window),
      size.width,
      size.height,
    )) {
      Ok(renderer) => renderer,
      Err(err) => {
        return Err(EvalError::Generic(format!(
          "Failed to create renderer: {}",
          err
        )));
      }
    };

    self.window.replace(Some(window));
    self.renderer.replace(Some(renderer));
    Ok(())
  }

  fn redraw(&self) -> Result<(), Box<dyn Error>> {
    let window = self.window.borrow();
    let Some(window) = window.as_ref() else {
      return Ok(());
    };

    self.scene.borrow_mut().reset();

    // TODO: Track packed widgets and layout instead of drawing every registered widget.
    let transform = Affine::scale(window.scale_factor());
    for widget in self.widgets.borrow().values() {
      widget
        .borrow()
        .redraw(&mut self.scene.borrow_mut(), transform);
    }

    let size = window.inner_size();

    self
      .renderer
      .borrow_mut()
      .as_mut()
      .map(|renderer| renderer.render(&self.scene.borrow(), size.width, size.height))
      .unwrap_or(Ok(()))
  }

  fn request_redraw(&self) {
    if let Some(window) = self.window.borrow().as_ref() {
      window.request_redraw();
    }
  }

  /// Whether a window is open (or is requested and pending creation).
  pub fn has_window(&self) -> bool {
    self.window_attributes.borrow().is_some()
  }

  pub fn handle_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if let Err(err) = self.ensure_window(event_loop) {
      eprintln!("{}", err);
      self.window_attributes.replace(None);
    }
  }

  pub fn handle_about_to_wait(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if let Err(err) = self.ensure_window(event_loop) {
      eprintln!("{}", err);
      self.window_attributes.replace(None);
    }
  }

  pub fn handle_window_event(
    &mut self,
    _window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
    tcl_event_loop: Rc<RefCell<tanaid::event_loop::EventLoop>>,
  ) {
    match event {
      WindowEvent::Resized(_) => {
        self.request_redraw();
      }
      WindowEvent::ScaleFactorChanged {
        scale_factor,
        mut inner_size_writer,
      } => {
        // Keep the logical size `pack` requested. If the scale goes A -> B -> A before the A -> B
        // resize lands (e.g. a new window briefly given another monitor's scale), winit pre-fills
        // the writer from the stale size it last saw; overwriting it stops winit sending that. But
        // winit skips the writer's resize when it equals that stale size, so the in-flight A -> B
        // resize would still win; `window.request_inner_size` always sends. Known gap: a scale
        // change undoes manual resizes.
        let requested = self
          .window_attributes
          .borrow()
          .as_ref()
          .and_then(|a| a.inner_size);
        if let Some(size) = requested {
          let size = size.to_physical::<u32>(scale_factor);
          let _ = inner_size_writer.request_inner_size(size);
          if let Some(window) = self.window.borrow().as_ref() {
            let _ = window.request_inner_size(size);
          }
        }
      }
      WindowEvent::RedrawRequested => {
        if let Err(err) = self.redraw() {
          eprintln!("Draw error: {}", err);
        }
        // TODO: only when dirty
        self.request_redraw();
      }
      WindowEvent::CloseRequested => {
        self.window_attributes.replace(None);
        self.window.replace(None);
        self.renderer.replace(None);
      }
      WindowEvent::ModifiersChanged(mods) => {
        self.event_bindings.borrow_mut().handle_modifiers(mods)
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if let Err(r) =
          self
            .event_bindings
            .borrow()
            .handle_key_event(".".to_string(), event, tcl_event_loop)
        {
          eprintln!("Error: {}", r);
        }
      }
      _ => {}
    }
  }
}
