use crate::color::Color;

pub type ItemId = i64;

/// Real Tk defaults to `-width 10c -height 7c`; we use round pixel counts
/// instead of screen distances, which aren't implemented.
pub const DEFAULT_WIDTH: u32 = 400;
pub const DEFAULT_HEIGHT: u32 = 300;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
  pub x1: f64,
  pub y1: f64,
  pub x2: f64,
  pub y2: f64,
}

#[derive(Clone, Debug)]
pub enum Shape {
  Rectangle { coords: Rect, fill: Option<Color> },
}

#[derive(Clone, Debug)]
pub struct Item {
  pub id: ItemId,
  pub shape: Shape,
}

/// The display list behind one `canvas` widget.
#[derive(Clone, Debug)]
pub struct Canvas {
  pub width: u32,
  pub height: u32,
  pub background: Color,
  items: Vec<Item>,
  next_item_id: ItemId,
}

impl Rect {
  /// Tk stores the coordinates as given, but draws the rectangle they span.
  pub fn normalized(&self) -> Rect {
    Rect {
      x1: self.x1.min(self.x2),
      y1: self.y1.min(self.y2),
      x2: self.x1.max(self.x2),
      y2: self.y1.max(self.y2),
    }
  }

  pub fn translated(&self, dx: f64, dy: f64) -> Rect {
    Rect {
      x1: self.x1 + dx,
      y1: self.y1 + dy,
      x2: self.x2 + dx,
      y2: self.y2 + dy,
    }
  }

  pub fn to_array(self) -> [f64; 4] {
    [self.x1, self.y1, self.x2, self.y2]
  }
}

impl Canvas {
  pub fn new(width: u32, height: u32, background: Color) -> Canvas {
    Canvas {
      width,
      height,
      background,
      items: Vec::new(),
      // Tk numbers canvas items from 1.
      next_item_id: 1,
    }
  }

  pub fn items(&self) -> &[Item] {
    &self.items
  }

  pub fn create_rectangle(&mut self, coords: Rect, fill: Option<Color>) -> ItemId {
    let id = self.next_item_id;
    self.next_item_id += 1;
    self.items.push(Item {
      id,
      shape: Shape::Rectangle { coords, fill },
    });
    id
  }

  pub fn item_mut(&mut self, id: ItemId) -> Option<&mut Item> {
    self.items.iter_mut().find(|item| item.id == id)
  }

  pub fn item(&self, id: ItemId) -> Option<&Item> {
    self.items.iter().find(|item| item.id == id)
  }

  pub fn item_ids(&self) -> Vec<ItemId> {
    self.items.iter().map(|item| item.id).collect()
  }

  pub fn delete(&mut self, id: ItemId) {
    self.items.retain(|item| item.id != id);
  }
}

impl Item {
  pub fn coords(&self) -> Rect {
    match self.shape {
      Shape::Rectangle { coords, .. } => coords,
    }
  }

  pub fn set_coords(&mut self, new_coords: Rect) {
    match &mut self.shape {
      Shape::Rectangle { coords, .. } => *coords = new_coords,
    }
  }

  pub fn set_fill(&mut self, new_fill: Option<Color>) {
    match &mut self.shape {
      Shape::Rectangle { fill, .. } => *fill = new_fill,
    }
  }
}
