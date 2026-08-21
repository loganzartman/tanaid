use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;

use crate::canvas::Canvas;

#[cfg(not(target_family = "wasm"))]
mod app;
mod canvas;
mod cmd_canvas;
mod cmd_pack;
mod cmd_widget;
mod color;
mod render;
#[cfg(test)]
mod tests;
mod web;

#[cfg(not(target_family = "wasm"))]
pub use app::run;
pub use canvas::{Canvas as CanvasWidget, Item, ItemId, Rect, Shape};
pub use color::Color;
pub use render::{Renderer, SoftwareRendering};
pub use web::create_renderer;

/// A handle to the Tk widget tree, shared by the commands registered on an
/// interpreter and by whoever is drawing the window.
#[derive(Clone)]
pub struct Tk {
  pub(crate) state: Rc<RefCell<TkState>>,
}

pub(crate) struct TkState {
  pub(crate) canvases: HashMap<String, Canvas>,
  /// Path name of the packed widget. Only one widget can be mapped for now, so
  /// there is no geometry manager to speak of.
  pub(crate) mapped: Option<String>,
  pub(crate) dirty: bool,
}

#[derive(Debug)]
pub enum TkError {
  Eval(EvalError),
  Graphics(String),
  Windowing(String),
}

impl Tk {
  pub fn new() -> Tk {
    Tk {
      state: Rc::new(RefCell::new(TkState {
        canvases: HashMap::new(),
        mapped: None,
        dirty: false,
      })),
    }
  }

  /// Registers the Tk commands (`canvas`, `pack`) on an interpreter. Widget
  /// commands like `.c` are registered as the widgets are created.
  pub fn register_commands(&self, context: &mut EvalContext) {
    let canvas_tk = self.clone();
    context.register_command(
      "canvas",
      Rc::new(move |args, context, _frame| cmd_canvas::eval(&canvas_tk, args, context)),
    );

    let pack_tk = self.clone();
    context.register_command(
      "pack",
      Rc::new(move |args, _context, _frame| cmd_pack::eval(&pack_tk, args)),
    );
  }

  /// Whether a widget has been mapped, i.e. whether there is a window to show.
  pub fn has_window(&self) -> bool {
    self.state.borrow().mapped.is_some()
  }

  /// The canvas to draw, if one has been packed.
  pub fn mapped_canvas(&self) -> Option<Ref<'_, Canvas>> {
    Ref::filter_map(self.state.borrow(), |state| {
      state.canvases.get(state.mapped.as_ref()?)
    })
    .ok()
  }

  /// Forgets the mapped widget, as though its window had been destroyed. The
  /// widgets themselves stick around, so a REPL can go on using them.
  pub fn unmap_widgets(&self) {
    let mut state = self.state.borrow_mut();
    state.mapped = None;
    state.dirty = false;
  }

  /// Whether the widget tree changed since the last call.
  pub fn take_dirty(&self) -> bool {
    let mut state = self.state.borrow_mut();
    std::mem::take(&mut state.dirty)
  }
}

impl Default for Tk {
  fn default() -> Tk {
    Tk::new()
  }
}

impl std::fmt::Display for TkError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TkError::Eval(err) => write!(f, "{}", err),
      TkError::Graphics(message) => write!(f, "Graphics error: {}", message),
      TkError::Windowing(message) => write!(f, "Windowing error: {}", message),
    }
  }
}

impl std::error::Error for TkError {}

impl From<EvalError> for TkError {
  fn from(err: EvalError) -> TkError {
    TkError::Eval(err)
  }
}
