use crate::item::canvas_item::CanvasItem;
use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::{self, Affine};
use vello::peniko::{Color, Fill};

#[derive(Default)]
pub struct CanvasItemRect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
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
