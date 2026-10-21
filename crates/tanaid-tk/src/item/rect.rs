use crate::item::canvas_item::{CanvasItem, CanvasItemCommonOptions};
use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::{self, Affine};
use vello::peniko::Fill;

#[derive(Default)]
pub struct CanvasItemRect {
  pub rect: kurbo::Rect,
  pub common_opts: CanvasItemCommonOptions,
}

impl CanvasItem for CanvasItemRect {
  fn common_options(&mut self) -> &mut CanvasItemCommonOptions {
    &mut self.common_opts
  }

  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    scene.fill(
      Fill::NonZero,
      transform,
      &self.common_opts.fill_brush,
      None,
      &self.rect,
    );
    if self.common_opts.stroke.width > 0. {
      scene.stroke(
        &self.common_opts.stroke,
        transform,
        &self.common_opts.stroke_brush,
        None,
        &self.rect,
      );
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
