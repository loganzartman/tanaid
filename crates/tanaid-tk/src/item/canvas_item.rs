use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::Affine;

pub trait CanvasItem {
  /// Render the item in the scene
  fn redraw(&self, scene: &mut Scene, transform: Affine);
  /// Get the coordinates of the item
  fn get_coords(&self) -> Vec<f64>;
  /// Set the coordinates of the item. Errors on invalid coordinates.
  fn set_coords(&mut self, coords: &[f64]) -> Result<(), EvalError>;
}
