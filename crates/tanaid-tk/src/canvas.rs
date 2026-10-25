use crate::{item::canvas_item::CanvasItem, widget::Widget};
use indexmap::IndexMap;
use vello::{
  Scene,
  kurbo::{Affine, Rect},
  peniko::{Brush, Color, Fill},
};

pub struct CanvasWidget {
  pub attrs: CanvasAttributes,
  pub items: IndexMap<i64, Box<dyn CanvasItem>>,
}

impl CanvasWidget {
  pub fn new(attrs: CanvasAttributes) -> Self {
    Self {
      attrs,
      items: IndexMap::new(),
    }
  }
}

impl Widget for CanvasWidget {
  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    let (width, height) = self.requested_size();

    const DEFAULT_BACKGROUND: Brush = Brush::Solid(Color::BLACK);
    scene.fill(
      Fill::NonZero,
      transform,
      self
        .attrs
        .background
        .as_ref()
        .unwrap_or(&DEFAULT_BACKGROUND),
      None,
      &Rect::new(0., 0., width as f64, height as f64),
    );

    for item in self.items.values() {
      item.redraw(scene, transform);
    }
  }

  fn requested_size(&self) -> (u32, u32) {
    (
      self.attrs.width.unwrap_or(256),
      self.attrs.height.unwrap_or(256),
    )
  }
}

pub struct CanvasAttributes {
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub background: Option<Brush>,
}

impl CanvasAttributes {
  pub fn new() -> Self {
    Self {
      width: None,
      height: None,
      background: None,
    }
  }
}
