use crate::color::parse_color;
use crate::item::canvas_item::CanvasItem;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;
use vello::Scene;
use vello::kurbo::{Affine, BezPath, Cap, Join, Stroke};
use vello::peniko::{Brush, Color};

pub struct CanvasItemLine {
  pub coords: Vec<f64>,
  path: BezPath,
  smooth: Smooth,
  fill: Option<Brush>,
  stroke: Stroke,
}

impl Default for CanvasItemLine {
  fn default() -> Self {
    Self {
      coords: Default::default(),
      path: Default::default(),
      smooth: Default::default(),
      fill: Some(Brush::Solid(Color::BLACK)),
      stroke: Stroke::new(1.).with_caps(Cap::Butt).with_join(Join::Round),
    }
  }
}

#[derive(Default)]
enum Smooth {
  #[default]
  None,
  Bezier,
  Raw,
}

impl CanvasItem for CanvasItemLine {
  fn apply_option(&mut self, name: &str, value: &mut Value) -> Result<bool, EvalError> {
    match name {
      "-fill" => {
        self.fill = parse_color(value.repr_str()?)?.map(|c| c.into());
      }
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
      "-width" => {
        self.stroke.width = value.repr_float()?;
      }
      _ => return Ok(false),
    }
    Ok(true)
  }

  fn redraw(&self, scene: &mut Scene, transform: Affine) {
    if let Some(fill) = &self.fill {
      scene.stroke(&self.stroke, transform, fill, None, &self.path);
    }
  }

  fn recompute(&mut self) -> Result<(), EvalError> {
    match self.smooth {
      Smooth::None => self.compute_path_smooth_false(),
      Smooth::Bezier => self.compute_path_smooth_bezier(),
      Smooth::Raw => self.compute_path_smooth_raw(),
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
    let (pairs, _) = self.coords.as_chunks::<2>();

    if pairs.len() < 3 {
      return self.compute_path_smooth_false();
    }

    let closed = pairs.first() == pairs.last();
    let point_triples = closed
      .then(|| [pairs[pairs.len() - 2], pairs[0], pairs[1]])
      .into_iter()
      .chain(pairs.array_windows::<3>().copied());

    self.path = BezPath::new();
    let n_open_triples = pairs.len() - 2;
    for (i, [[ax, ay], [bx, by], [cx, cy]]) in point_triples.enumerate() {
      let first = i == 0;
      let last = i == n_open_triples - 1;

      if first {
        let p0 = if closed {
          ((ax + bx) / 2., (ay + by) / 2.)
        } else {
          (ax, ay)
        };
        self.path.move_to(p0);
      }

      let p1 = if !closed && first {
        (ax / 3. + 2. * bx / 3., ay / 3. + 2. * by / 3.)
      } else {
        (ax / 6. + 5. * bx / 6., ay / 6. + 5. * by / 6.)
      };

      let p2 = if !closed && last {
        (2. * bx / 3. + cx / 3., 2. * by / 3. + cy / 3.)
      } else {
        (5. * bx / 6. + cx / 6., 5. * by / 6. + cy / 6.)
      };

      let p3 = if !closed && last {
        (cx, cy)
      } else {
        ((bx + cx) / 2., (by + cy) / 2.)
      };

      self.path.curve_to(p1, p2, p3);
    }
  }

  fn compute_path_smooth_raw(&mut self) {
    let (pairs, _) = self.coords.as_chunks::<2>();

    if pairs.len() < 3 {
      return self.compute_path_smooth_false();
    }

    let n = pairs.len();
    let p = |i: usize| {
      let [x, y] = pairs[i % n];
      (x, y)
    };

    self.path = BezPath::new();
    self.path.move_to(p(0));
    for k in (0..n - 1).step_by(3) {
      self.path.curve_to(p(k + 1), p(k + 2), p(k + 3));
    }
  }
}
