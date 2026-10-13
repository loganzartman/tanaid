mod key;

use crate::{
  tcl::Tcl,
  tk::key::{TclModifierKeys, key_from_js, key_location_from_js, modifiers_from_js},
};
use std::{cell::RefCell, error::Error, rc::Rc};
use tanaid::event_loop::EventLoop;
use tanaid_tk::{SurfaceTarget, events::TclEventType, keysym::key_detail};
use tsify::Ts;
use wasm_bindgen::{JsError, prelude::wasm_bindgen};
use web_sys::OffscreenCanvas;

#[wasm_bindgen]
pub struct Tk {
  tk: tanaid_tk::Tk,
  event_loop: Option<Rc<RefCell<EventLoop>>>,
}

#[wasm_bindgen]
impl Tk {
  pub fn create() -> Self {
    Tk {
      tk: tanaid_tk::Tk::new(),
      event_loop: None,
    }
  }

  /// Install Tk into the given Tcl interpreter. If the interpreter is running a script, returns an error.
  pub fn install(&mut self, tcl: &mut Tcl) -> Result<(), JsError> {
    let mut context = tcl.interpreter.take_context()?;
    self.event_loop = Some(Rc::clone(&context.event_loop));
    self.tk.install(&mut context)?;
    tcl.interpreter.configure(context);
    Ok(())
  }

  #[wasm_bindgen(js_name = "attachCanvas")]
  pub async fn attach_canvas(&self, canvas: OffscreenCanvas) -> Result<(), JsError> {
    self
      .tk
      .context
      .attach_surface_target(SurfaceTarget::OffscreenCanvas(canvas))
      .await?;
    Ok(())
  }

  pub fn redraw(&self, scale_factor: f64) -> Result<(), JsError> {
    self
      .tk
      .context
      .redraw_requested_size(scale_factor)
      .map_err(std_error_to_error)
  }

  #[wasm_bindgen(js_name = "hasWindow")]
  pub fn has_window(&self) -> bool {
    self.tk.context.has_window()
  }

  #[wasm_bindgen(js_name = "handleKeyDown")]
  pub fn handle_key_down(
    &self,
    tag: &str,
    key: &str,
    location: u32,
    modifiers: Ts<TclModifierKeys>,
  ) -> Result<(), JsError> {
    self.handle_key_event(tag, TclEventType::KeyPress, key, location, modifiers)
  }

  #[wasm_bindgen(js_name = "handleKeyUp")]
  pub fn handle_key_up(
    &self,
    tag: &str,
    key: &str,
    location: u32,
    modifiers: Ts<TclModifierKeys>,
  ) -> Result<(), JsError> {
    self.handle_key_event(tag, TclEventType::KeyRelease, key, location, modifiers)
  }

  fn handle_key_event(
    &self,
    tag: &str,
    event_type: TclEventType,
    key: &str,
    location: u32,
    modifiers: Ts<TclModifierKeys>,
  ) -> Result<(), JsError> {
    if let Some(event_loop) = &self.event_loop {
      let detail = key_detail(&key_from_js(key), key_location_from_js(location));
      let modifiers = modifiers_from_js(modifiers.to_rust()?);
      self.tk.context.handle_event(
        Rc::clone(event_loop),
        tag,
        event_type,
        detail.as_str(),
        modifiers,
      )?;
    }
    Ok(())
  }
}

fn std_error_to_error(value: Box<dyn Error>) -> JsError {
  JsError::new(value.to_string().as_str())
}
