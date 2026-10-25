use crate::color::parse_color;
use crate::item::canvas_item::CanvasItem;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use vello::Scene;
use vello::kurbo::{self, Affine, Stroke};
use vello::peniko::{Brush, Fill};

#[derive(Default)]
pub struct CanvasItemRect {
  pub rect: kurbo::Rect,
  fill: Brush,
  outline: Brush,
  stroke: Stroke,
}

impl CanvasItem for CanvasItemRect {
  fn apply_option(&mut self, name: &str, value: &mut Value) -> Result<bool, EvalError> {
    match name {
      "-fill" => {
        self.fill = parse_color(value.repr_str()?)?.into();
      }
      "-outline" => {
        self.outline = parse_color(value.repr_str()?)?.into();
      }
      "-width" => {
        self.stroke.width = value.repr_float()?;
      }
      _ => return Ok(false),
    }
    Ok(true)
  }

  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    scene.fill(Fill::NonZero, transform, &self.fill, None, &self.rect);
    if self.stroke.width > 0. {
      scene.stroke(&self.stroke, transform, &self.outline, None, &self.rect);
    }
  }

  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError> {
    let [x1, y1, x2, y2] = coords else {
      return Err(EvalError::Generic(
        "wrong number of coords: rect expects 4".to_string(),
      ));
    };
    self.rect.x0 = x1.min(*x2);
    self.rect.y0 = y1.min(*y2);
    self.rect.x1 = x1.max(*x2);
    self.rect.y1 = y1.max(*y2);
    Ok(())
  }

  fn get_coords(&self) -> Vec<f64> {
    vec![self.rect.x0, self.rect.y0, self.rect.x1, self.rect.y1]
  }
}
