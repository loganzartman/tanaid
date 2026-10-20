use crate::item::canvas_item::CanvasItem;
use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Stroke};
use vello::peniko::Color;

#[derive(Default)]
pub struct CanvasItemLine {
  pub coords: Vec<f64>,
  path: BezPath,
}

impl CanvasItem for CanvasItemLine {
  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    scene.stroke(
      &Stroke::new(1.0),
      transform,
      Color::new([1., 1., 1., 1.]),
      None,
      &self.path,
    );
  }

  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError> {
    if coords.len() < 4 {
      return Err(EvalError::Generic(
        "wrong number of coords: line expects at least two pairs".to_string(),
      ));
    }
    if coords.len() % 2 == 1 {
      return Err(EvalError::Generic(
        "wrong number of coords: line expects pairs".to_string(),
      ));
    }

    self.path = BezPath::new();
    let (pairs, _) = coords.as_chunks::<2>();
    if let Some(([x, y], rest)) = pairs.split_first() {
      self.path.move_to((*x, *y));
      for [x, y] in rest {
        self.path.line_to((*x, *y));
      }
    }

    self.coords = Vec::from(coords);
    Ok(())
  }

  fn get_coords(&self) -> Vec<f64> {
    self.coords.clone()
  }
}
