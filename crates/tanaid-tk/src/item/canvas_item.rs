use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use vello::Scene;
use vello::kurbo::{Affine, Stroke};
use vello::peniko::{Brush, Color};

use crate::color::parse_color;

pub trait CanvasItem {
  /// Get a mutable CanvasItemCommonOptions for this item
  fn common_options(&mut self) -> &mut CanvasItemCommonOptions;
  /// Override to handle non-common options
  fn apply_own_option(&mut self, _name: &str, _value: &mut Value) -> Result<bool, EvalError> {
    Ok(false)
  }
  /// Render the item in the scene
  fn redraw(&self, scene: &mut Scene, transform: Affine);
  /// Get the coordinates of the item
  fn get_coords(&self) -> Vec<f64>;
  /// Set the coordinates of the item. Errors on invalid coordinates.
  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError>;

  fn apply_common_option(&mut self, name: &str, value: &mut Value) -> Result<bool, EvalError> {
    self.common_options().apply_option(name, value)
  }

  fn with_coords(mut self, coords: &[f64]) -> Result<Self, EvalError>
  where
    Self: Sized,
  {
    self.set_coords(coords)?;
    Ok(self)
  }
}

pub struct CanvasItemCommonOptions {
  pub fill_brush: Brush,
  pub stroke_brush: Brush,
  pub stroke: Stroke,
}

impl Default for CanvasItemCommonOptions {
  fn default() -> Self {
    Self {
      fill_brush: Color::new([1., 1., 1., 0.]).into(),
      stroke_brush: Color::new([1., 1., 1., 1.]).into(),
      stroke: Stroke::new(1.0),
    }
  }
}

impl CanvasItemCommonOptions {
  pub fn apply_option(&mut self, name: &str, value: &mut Value) -> Result<bool, EvalError> {
    match name {
      "-fill" => self.fill_brush = Color::new(parse_color(value.repr_str()?)?.into()).into(),
      "-outline" => self.stroke_brush = Color::new(parse_color(value.repr_str()?)?.into()).into(),
      "-width" => self.stroke.width = value.repr_float()?,
      _ => return Ok(false),
    }
    Ok(true)
  }
}
