struct Viewport {
  // size of the drawable area, in pixels
  size: vec2<f32>,
  padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> viewport: Viewport;

struct RectInstance {
  // (x1, y1, x2, y2) in canvas pixels: origin top left, y pointing down
  @location(0) bounds: vec4<f32>,
  @location(1) color: vec4<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) color: vec4<f32>,
}

// Each instance is drawn as a 4 vertex triangle strip, so the vertex index
// picks out one corner of the rectangle.
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: RectInstance) -> VertexOutput {
  let corner = vec2<f32>(f32(vertex_index & 1u), f32((vertex_index >> 1u) & 1u));
  let position = mix(instance.bounds.xy, instance.bounds.zw, corner);

  var out: VertexOutput;
  out.clip_position = vec4<f32>(
    position.x / viewport.size.x * 2.0 - 1.0,
    1.0 - position.y / viewport.size.y * 2.0,
    0.0,
    1.0,
  );
  out.color = instance.color;
  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.color;
}
