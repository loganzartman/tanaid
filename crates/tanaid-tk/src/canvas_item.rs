use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::{self, Affine};
use vello::peniko::{Color, Fill};

pub trait CanvasItem {
  /// Render the item in the scene
  fn redraw(&self, scene: &mut Scene, transform: Affine);
  /// Get the coordinates of the item
  fn get_coords(&self) -> Vec<f64>;
  /// Set the coordinates of the item. Errors on invalid coordinates.
  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError>;
}

pub struct CanvasItemRect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

impl CanvasItemRect {
  pub fn new() -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      width: 0.0,
      height: 0.0,
    }
  }

  pub fn with_coords(mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<Self, EvalError> {
    self.set_coords(&[x1, y1, x2, y2])?;
    Ok(self)
  }
}

impl CanvasItem for CanvasItemRect {
  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    scene.fill(
      Fill::NonZero,
      transform,
      Color::new([1., 1., 1., 1.]),
      None,
      &kurbo::Rect::new(self.x, self.y, self.x + self.width, self.y + self.height),
    );
  }

  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError> {
    let [x1, y1, x2, y2] = coords else {
      return Err(EvalError::Generic(
        "wrong number of coords: rect expects 4".to_string(),
      ));
    };
    self.x = x1.min(*x2);
    self.y = y1.min(*y2);
    self.width = x2.max(*x1) - self.x;
    self.height = y2.max(*y1) - self.y;
    Ok(())
  }

  fn get_coords(&self) -> Vec<f64> {
    vec![self.x, self.y, self.x + self.width, self.y + self.height]
  }
}
