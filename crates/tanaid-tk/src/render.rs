use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;

use crate::TkError;
use crate::canvas::{Canvas, Shape};
use crate::color::Color;

/// One filled rectangle, as uploaded to the GPU.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RectInstance {
  bounds: [f32; 4],
  color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Viewport {
  size: [f32; 2],
  padding: [f32; 2],
}

const INITIAL_INSTANCE_CAPACITY: u64 = 64;

/// Whether a software (CPU) adapter will do. A software rasterizer is a decent
/// last resort, but it loses to any hardware backend when both are present.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftwareRendering {
  Allowed,
  Rejected,
}

/// Draws a [`Canvas`] display list onto a surface.
pub struct Renderer {
  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  pipeline: wgpu::RenderPipeline,
  viewport_buffer: wgpu::Buffer,
  viewport_bind_group: wgpu::BindGroup,
  instance_buffer: wgpu::Buffer,
  instance_capacity: u64,
  instances: Vec<RectInstance>,
  /// Errors wgpu reports outside of a call we can get a result from. Without a
  /// handler of our own, wgpu panics the process on these.
  uncaptured_error: Arc<Mutex<Option<String>>>,
}

impl Renderer {
  pub async fn new(
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    width: u32,
    height: u32,
    software_rendering: SoftwareRendering,
  ) -> Result<Renderer, TkError> {
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
      })
      .await
      .map_err(|e| TkError::Graphics(format!("no usable graphics adapter: {}", e)))?;

    if software_rendering == SoftwareRendering::Rejected
      && adapter.get_info().device_type == wgpu::DeviceType::Cpu
    {
      return Err(TkError::Graphics(
        "only a software adapter is available".to_string(),
      ));
    }

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        label: Some("tanaid-tk"),
        required_features: wgpu::Features::empty(),
        // the lowest common denominator, so that the same renderer works in a
        // browser over WebGL
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        ..Default::default()
      })
      .await
      .map_err(|e| TkError::Graphics(format!("failed to request device: {}", e)))?;

    let uncaptured_error = Arc::new(Mutex::new(None));
    watch_for_uncaptured_errors(&device, &uncaptured_error);

    let config = surface
      .get_default_config(&adapter, width.max(1), height.max(1))
      .ok_or_else(|| TkError::Graphics("surface is not supported by the adapter".to_string()))?;
    configure_surface(&device, &surface, &config).await?;

    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("viewport"),
      contents: bytemuck::bytes_of(&Viewport {
        size: [config.width as f32, config.height as f32],
        padding: [0., 0.],
      }),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let viewport_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("viewport"),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
    });

    let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("viewport"),
      layout: &viewport_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: viewport_buffer.as_entire_binding(),
      }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("rectangles"),
      bind_group_layouts: &[Some(&viewport_layout)],
      immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("rectangles"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        compilation_options: Default::default(),
        buffers: &[Some(wgpu::VertexBufferLayout {
          array_stride: size_of::<RectInstance>() as wgpu::BufferAddress,
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
        })],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: config.format,
          blend: Some(wgpu::BlendState::ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleStrip,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: Default::default(),
      multiview_mask: None,
      cache: None,
    });

    let instance_buffer = create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);

    Ok(Renderer {
      surface,
      device,
      queue,
      config,
      pipeline,
      viewport_buffer,
      viewport_bind_group,
      instance_buffer,
      instance_capacity: INITIAL_INSTANCE_CAPACITY,
      instances: Vec::new(),
      uncaptured_error,
    })
  }

  /// Checks that the surface really hands out frames.
  ///
  /// A driver can accept the configuration and then never present: WSL's Vulkan
  /// claims the window's surface but cannot draw to it. Callers that have
  /// another backend to fall back to ask for one frame up front; a browser,
  /// which has only the one, doesn't.
  pub fn probe_present(&mut self) -> Result<(), TkError> {
    match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
        Ok(())
      }
      status => Err(TkError::Graphics(format!(
        "surface cannot present: {:?}",
        status
      ))),
    }
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    if width == 0 || height == 0 || (width == self.config.width && height == self.config.height) {
      return;
    }

    self.config.width = width;
    self.config.height = height;
    self.surface.configure(&self.device, &self.config);
  }

  pub fn render(&mut self, canvas: &Canvas) -> Result<(), TkError> {
    self.write_instances(canvas);

    self.queue.write_buffer(
      &self.viewport_buffer,
      0,
      bytemuck::bytes_of(&Viewport {
        size: [self.config.width as f32, self.config.height as f32],
        padding: [0., 0.],
      }),
    );

    let Some(frame) = self.current_texture()? else {
      return Ok(());
    };
    let view = frame
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("canvas"),
      });

    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("canvas"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(self.clear_color(canvas.background)),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });

      pass.set_pipeline(&self.pipeline);
      pass.set_bind_group(0, &self.viewport_bind_group, &[]);
      pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
      pass.draw(0..4, 0..self.instances.len() as u32);
    }

    self.queue.submit([encoder.finish()]);
    self.queue.present(frame);

    self.take_uncaptured_error()
  }

  fn take_uncaptured_error(&self) -> Result<(), TkError> {
    match lock(&self.uncaptured_error).take() {
      Some(message) => Err(TkError::Graphics(message)),
      None => Ok(()),
    }
  }

  /// Rebuilds the instance buffer from the canvas display list.
  fn write_instances(&mut self, canvas: &Canvas) {
    self.instances.clear();
    for item in canvas.items() {
      match item.shape {
        Shape::Rectangle { coords, fill } => {
          let Some(fill) = fill else {
            // an unfilled rectangle is all outline, which isn't drawn yet
            continue;
          };
          let bounds = coords.normalized();
          self.instances.push(RectInstance {
            bounds: [
              bounds.x1 as f32,
              bounds.y1 as f32,
              bounds.x2 as f32,
              bounds.y2 as f32,
            ],
            color: self.encode_color(fill),
          });
        }
      }
    }

    if self.instances.is_empty() {
      return;
    }

    if self.instances.len() as u64 > self.instance_capacity {
      self.instance_capacity = (self.instances.len() as u64).next_power_of_two();
      self.instance_buffer = create_instance_buffer(&self.device, self.instance_capacity);
    }

    self.queue.write_buffer(
      &self.instance_buffer,
      0,
      bytemuck::cast_slice(&self.instances),
    );
  }

  fn current_texture(&mut self) -> Result<Option<wgpu::SurfaceTexture>, TkError> {
    use wgpu::CurrentSurfaceTexture::*;

    // one retry: a surface that went stale (resize, monitor change) is usable
    // again once reconfigured
    for _ in 0..2 {
      match self.surface.get_current_texture() {
        Success(frame) | Suboptimal(frame) => return Ok(Some(frame)),
        Outdated | Lost => self.surface.configure(&self.device, &self.config),
        Timeout | Occluded => return Ok(None),
        Validation => {
          return Err(TkError::Graphics("surface is no longer valid".to_string()));
        }
      }
    }

    Ok(None)
  }

  fn encode_color(&self, color: Color) -> [f32; 4] {
    if self.config.format.is_srgb() {
      color.to_linear_array()
    } else {
      color.to_array()
    }
  }

  fn clear_color(&self, color: Color) -> wgpu::Color {
    let [r, g, b, a] = self.encode_color(color);
    wgpu::Color {
      r: r as f64,
      g: g as f64,
      b: b as f64,
      a: a as f64,
    }
  }
}

/// Records errors wgpu reports outside a call that could return them. Without a
/// handler, wgpu panics the process on those.
#[cfg(not(target_family = "wasm"))]
fn watch_for_uncaptured_errors(device: &wgpu::Device, errors: &Arc<Mutex<Option<String>>>) {
  let errors = errors.clone();
  device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
    let mut slot = lock(&errors);
    slot.get_or_insert_with(|| error.to_string());
  }));
}

/// In a browser, wgpu converts a raised GPU error into a `wgpu::Error` before
/// handing it to any handler, and panics on the kinds it doesn't map — Chrome's
/// `GPUInternalError` among them. Installing a handler there turns an error we
/// would have reported into an abort, so the browser keeps wgpu's own handling,
/// which logs the error and carries on.
#[cfg(target_family = "wasm")]
fn watch_for_uncaptured_errors(_device: &wgpu::Device, _errors: &Arc<Mutex<Option<String>>>) {}

/// Configures the surface, reporting a rejected configuration as an error: an
/// adapter can advertise a surface it turns out not to be able to present to,
/// and the caller may have another backend to try.
#[cfg(not(target_family = "wasm"))]
async fn configure_surface(
  device: &wgpu::Device,
  surface: &wgpu::Surface<'static>,
  config: &wgpu::SurfaceConfiguration,
) -> Result<(), TkError> {
  let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
  surface.configure(device, config);

  match error_scope.pop().await {
    Some(error) => Err(TkError::Graphics(format!(
      "failed to configure surface: {}",
      error
    ))),
    None => Ok(()),
  }
}

/// Popping an error scope in a browser panics on the same unmapped error kinds
/// as [`watch_for_uncaptured_errors`]. wgpu already reports a configuration the
/// browser rejected by marking the surface lost, which [`Renderer::render`]
/// handles, so there is nothing to catch here.
#[cfg(target_family = "wasm")]
async fn configure_surface(
  device: &wgpu::Device,
  surface: &wgpu::Surface<'static>,
  config: &wgpu::SurfaceConfiguration,
) -> Result<(), TkError> {
  surface.configure(device, config);
  Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
  // the only thing held across the lock is a String, so a panic can't have left
  // it inconsistent
  mutex.lock().unwrap_or_else(|err| err.into_inner())
}

fn create_instance_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
  device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("rectangles"),
    size: capacity * size_of::<RectInstance>() as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  })
}
