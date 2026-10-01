use crate::{canvas_item::CanvasItem, widget::Widget};
use indexmap::IndexMap;
use vello::{Scene, kurbo::Affine};

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
}

impl CanvasAttributes {
  pub fn new() -> Self {
    Self {
      width: None,
      height: None,
    }
  }
}
