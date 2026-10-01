use vello::{Scene, kurbo::Affine};

pub trait Widget {
  fn redraw(&self, scene: &mut Scene, transform: Affine);
  fn requested_size(&self) -> (u32, u32);
}
