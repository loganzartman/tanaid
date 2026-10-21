use crate::item::canvas_item::{CanvasItem, CanvasItemCommonOptions};
use tanaid::eval_error::EvalError;
use vello::Scene;
use vello::kurbo::{Affine, BezPath};

#[derive(Default)]
pub struct CanvasItemLine {
  pub coords: Vec<f64>,
  path: BezPath,
  common_options: CanvasItemCommonOptions,
  smooth: Smooth,
}

#[derive(Default)]
enum Smooth {
  #[default]
  None,
  Bezier,
  Raw,
}

impl CanvasItem for CanvasItemLine {
  fn common_options(&mut self) -> &mut CanvasItemCommonOptions {
    &mut self.common_options
  }

  fn apply_own_option(
    &mut self,
    name: &str,
    value: &mut tanaid::value::Value,
  ) -> Result<bool, EvalError> {
    match name {
      "-smooth" => {
        match value.repr_bool() {
          Ok(true) => {
            self.smooth = Smooth::Bezier;
            return Ok(true);
          }
          Ok(false) => {
            self.smooth = Smooth::None;
            return Ok(true);
          }
          Err(_) => {}
        }

        match value.repr_str()? {
          "bezier" => {
            self.smooth = Smooth::Bezier;
            return Ok(true);
          }
          "raw" => {
            self.smooth = Smooth::Raw;
            return Ok(true);
          }
          _ => {}
        }

        return Err(EvalError::ArgumentError(format!(
          "smooth: expected true, false, bezier, or raw; got: {value}"
        )));
      }
      "-splinesteps" => {
        // no need; our curves are literally perfect
      }
      _ => return Ok(false),
    }
    Ok(true)
  }

  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    scene.stroke(
      &self.common_options.stroke,
      transform,
      &self.common_options.fill_brush,
      None,
      &self.path,
    );
  }

  fn recompute(&mut self) -> Result<(), EvalError> {
    match self.smooth {
      Smooth::None => self.compute_path_smooth_false(),
      Smooth::Bezier => self.compute_path_smooth_bezier(),
      Smooth::Raw => self.compute_path_smooth_bezier(),
    }
    Ok(())
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

    self.coords = Vec::from(coords);
    Ok(())
  }

  fn get_coords(&self) -> Vec<f64> {
    self.coords.clone()
  }
}

impl CanvasItemLine {
  fn compute_path_smooth_false(&mut self) {
    self.path = BezPath::new();
    let (pairs, _) = self.coords.as_chunks::<2>();
    if let Some(([x, y], rest)) = pairs.split_first() {
      self.path.move_to((*x, *y));
      for [x, y] in rest {
        self.path.line_to((*x, *y));
      }
    }
  }

  fn compute_path_smooth_bezier(&mut self) {
    self.path = BezPath::new();
    let (pairs, _) = self.coords.as_chunks::<2>();

    if pairs.len() < 2 {
      return;
    }

    self.path.move_to((pairs[0][0], pairs[0][1]));
    if pairs.len() < 3 {
      self.path.line_to((pairs[1][0], pairs[1][1]));
      return;
    }

    let point_triples = pairs.windows(3);
    let n = point_triples.len();
    for (i, triple) in point_triples.enumerate() {
      let [[ax, ay], [bx, by], [cx, cy]] = triple else {
        unreachable!("must be triple");
      };

      let p1 = if i == 0 {
        (ax / 3. + 2. * bx / 3., ay / 3. + 2. * by / 3.)
      } else {
        (ax / 6. + 5. * bx / 6., ay / 6. + 5. * by / 6.)
      };

      let p2 = if i == n - 1 {
        (2. * bx / 3. + cx / 3., 2. * by / 3. + cy / 3.)
      } else {
        (5. * bx / 6. + cx / 6., 5. * by / 6. + cy / 6.)
      };

      let p3 = if i == n - 1 {
        (*cx, *cy)
      } else {
        ((bx + cx) / 2., (by + cy) / 2.)
      };

      self.path.curve_to(p1, p2, p3);
    }
  }
}
