use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use vello::Scene;
use vello::kurbo::Affine;

pub trait CanvasItem {
  /// Apply a configuration option if valid. Return false if the option is not supported.
  fn apply_option(&mut self, _name: &str, _value: &mut Value) -> Result<bool, EvalError> {
    Ok(false)
  }
  /// Render the item in the scene
  fn redraw(&self, scene: &mut Scene, transform: Affine);
  /// Update computed values
  fn recompute(&mut self) -> Result<(), EvalError> {
    Ok(())
  }
  /// Get the coordinates of the item
  fn get_coords(&self) -> Vec<f64>;
  /// Set the coordinates of the item. Errors on invalid coordinates.
  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError>;

  fn with_coords(mut self, coords: &[f64]) -> Result<Self, EvalError>
  where
    Self: Sized,
  {
    self.set_coords(coords)?;
    self.recompute()?;
    Ok(self)
  }
}
