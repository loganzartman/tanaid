use std::error::Error;

use vello::{
  AaConfig, RenderParams, Renderer, RendererOptions, Scene,
  peniko::color::palette,
  util::{RenderContext, RenderSurface},
  wgpu::{self, CurrentSurfaceTexture, PresentMode, SurfaceTarget},
};

pub struct TkRenderer {
  context: RenderContext,
  surface: RenderSurface<'static>,
  renderer: Renderer,
}

impl TkRenderer {
  pub async fn new(
    target: impl Into<SurfaceTarget<'static>>,
    width: u32,
    height: u32,
  ) -> Result<Self, Box<dyn Error>> {
    let mut context = RenderContext::new();
    let surface = context
      .create_surface(target, width, height, PresentMode::AutoNoVsync)
      .await?;

    let device_handle = &context.devices[surface.dev_id];
    let renderer = Renderer::new(&device_handle.device, RendererOptions::default())?;

    Ok(TkRenderer {
      context,
      surface,
      renderer,
    })
  }

  pub fn render(&mut self, scene: &Scene, width: u32, height: u32) -> Result<(), Box<dyn Error>> {
    if width == 0 || height == 0 {
      return Ok(());
    }

    if self.surface.config.width != width || self.surface.config.height != height {
      self
        .context
        .resize_surface(&mut self.surface, width, height);
    }

    let device_handle = &self.context.devices[self.surface.dev_id];

    // render to texture
    self.renderer.render_to_texture(
      &device_handle.device,
      &device_handle.queue,
      scene,
      &self.surface.target_view,
      &RenderParams {
        base_color: palette::css::BLACK,
        width,
        height,
        antialiasing_method: AaConfig::Area,
      },
    )?;

    // get next swapchain frame
    let frame = match self.surface.surface.get_current_texture() {
      CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => frame,
      CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
        self.context.configure_surface(&self.surface);
        return Ok(());
      }
      // skip frame
      CurrentSurfaceTexture::Timeout
      | CurrentSurfaceTexture::Occluded
      | CurrentSurfaceTexture::Validation => return Ok(()),
    };

    // copy texture to frame and present
    let mut encoder =
      device_handle
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("tk blit"),
        });
    self.surface.blitter.copy(
      &device_handle.device,
      &mut encoder,
      &self.surface.target_view,
      &frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default()),
    );
    device_handle.queue.submit([encoder.finish()]);

    frame.present();

    Ok(())
  }
}
