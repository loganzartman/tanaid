use crate::{tcl::Tcl, util::std_error_to_error};
use tanaid_tk::SurfaceTarget;
use wasm_bindgen::{JsError, prelude::wasm_bindgen};
use web_sys::OffscreenCanvas;

#[wasm_bindgen]
pub struct Tk {
  tk: tanaid_tk::Tk,
}

#[wasm_bindgen]
impl Tk {
  pub fn create() -> Self {
    Tk {
      tk: tanaid_tk::Tk::new(),
    }
  }

  /// Install Tk into the given Tcl interpreter. If the interpreter is running a script, returns an error.
  pub fn install(&mut self, tcl: &mut Tcl) -> Result<(), JsError> {
    let mut context = tcl.interpreter.take_context()?;
    self.tk.install(&mut context)?;
    tcl.interpreter.configure(context);
    Ok(())
  }

  #[wasm_bindgen(js_name = "attachCanvas")]
  pub async fn attach_canvas(&mut self, canvas: OffscreenCanvas) -> Result<(), JsError> {
    self
      .tk
      .context
      .attach_surface_target(SurfaceTarget::OffscreenCanvas(canvas))
      .await?;
    Ok(())
  }

  pub fn redraw(&mut self, scale_factor: f64) -> Result<(), JsError> {
    self
      .tk
      .context
      .redraw_requested_size(scale_factor)
      .map_err(std_error_to_error)
  }
}
