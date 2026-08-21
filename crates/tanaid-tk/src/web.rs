use web_sys::OffscreenCanvas;

use crate::TkError;
use crate::render::Renderer;
#[cfg(target_family = "wasm")]
use crate::render::SoftwareRendering;

/// Draws onto a canvas transferred from the page. A worker only ever sees an
/// `OffscreenCanvas`, which is also the only canvas wgpu can target from there.
#[cfg(target_family = "wasm")]
pub async fn create_renderer(canvas: OffscreenCanvas) -> Result<Renderer, TkError> {
  let (width, height) = (canvas.width(), canvas.height());

  // A browser can offer `navigator.gpu` and still have no WebGPU adapter
  // behind it, and the backend has to be chosen when the instance is created,
  // so this probes for a real adapter before falling back to WebGL.
  let instance = wgpu::util::new_instance_with_webgpu_detection(
    wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
  )
  .await;
  let surface = instance
    .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas))
    .map_err(|e| TkError::Graphics(e.to_string()))?;

  // the browser is the only adapter there is; take what it offers
  Renderer::new(instance, surface, width, height, SoftwareRendering::Allowed).await
}

/// Native builds of the browser bindings — `cargo build --workspace` compiles
/// them for the host too — have no browser to ask for a canvas.
#[cfg(not(target_family = "wasm"))]
pub async fn create_renderer(_canvas: OffscreenCanvas) -> Result<Renderer, TkError> {
  Err(TkError::Graphics(
    "a canvas can only be drawn on in a browser".to_string(),
  ))
}
