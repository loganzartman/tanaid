use tanaid::eval_error::EvalError;
use tanaid::value::Value;

use crate::Tk;
use crate::canvas::{Canvas, ItemId, Rect};
use crate::color::Color;

/// Evaluates a canvas widget command, e.g. `.c create rectangle 0 0 10 10`.
pub(crate) fn eval(tk: &Tk, path: &str, args: &mut [Value]) -> Result<Value, EvalError> {
  let [subcommand, rest @ ..] = args else {
    return Err(EvalError::ArgumentError(format!(
      "wrong number of args: expected \"{} option ?arg ...?\"",
      path
    )));
  };

  let subcommand = subcommand.repr_str()?.to_string();
  let mutates = match subcommand.as_str() {
    "create" | "delete" | "itemconfigure" | "move" => true,
    "coords" => rest.len() > 1,
    _ => false,
  };

  let mut state = tk.state.borrow_mut();
  let canvas = state
    .canvases
    .get_mut(path)
    .ok_or_else(|| EvalError::Generic(format!("internal error: no widget named {}", path)))?;

  let result = match subcommand.as_str() {
    "coords" => eval_coords(canvas, rest),
    "create" => eval_create(canvas, rest),
    "delete" => eval_delete(canvas, rest),
    "itemconfigure" => eval_itemconfigure(canvas, rest),
    "move" => eval_move(canvas, rest),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported canvas subcommand: {}",
      subcommand
    ))),
  };

  if result.is_ok() && mutates {
    state.dirty = true;
  }

  result
}

/// Resolves a tagOrId to the items it matches. Only item ids and the `all` tag
/// are supported; as in Tk, matching nothing is not an error.
fn resolve(canvas: &Canvas, tag_or_id: &mut Value) -> Result<Vec<ItemId>, EvalError> {
  let spec = tag_or_id.repr_str()?;
  if spec == "all" {
    return Ok(canvas.item_ids());
  }

  let id = spec.parse::<ItemId>().map_err(|_| {
    EvalError::ArgumentError(format!(
      "unsupported tag: \"{}\" (only item ids and \"all\" are supported)",
      spec
    ))
  })?;

  Ok(
    canvas
      .item(id)
      .map(|item| vec![item.id])
      .unwrap_or_default(),
  )
}

fn eval_create(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  let [item_type, rest @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"create type coords ?option value ...?\"".to_string(),
    ));
  };

  let item_type = item_type.repr_str()?.to_string();
  match item_type.as_str() {
    "rectangle" => eval_create_rectangle(canvas, rest),
    _ => Err(EvalError::ArgumentError(format!(
      "unsupported canvas item type: {}",
      item_type
    ))),
  }
}

fn eval_create_rectangle(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  let [x1, y1, x2, y2, options @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"create rectangle x1 y1 x2 y2 ?option value ...?\""
        .to_string(),
    ));
  };

  let coords = Rect {
    x1: x1.repr_float()?,
    y1: y1.repr_float()?,
    x2: x2.repr_float()?,
    y2: y2.repr_float()?,
  };

  // Tk leaves rectangles unfilled unless asked.
  let mut fill = None;
  for option in options.chunks_mut(2) {
    let [name, value] = option else {
      return Err(EvalError::ArgumentError(format!(
        "value for \"{}\" missing",
        option[0].repr_str()?
      )));
    };

    let name = name.repr_str()?.to_string();
    match name.as_str() {
      "-fill" => fill = optional_color(value)?,
      _ => {
        return Err(EvalError::ArgumentError(format!(
          "unsupported rectangle option: {}",
          name
        )));
      }
    }
  }

  Ok(Value::from(canvas.create_rectangle(coords, fill)))
}

fn eval_coords(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  let [tag_or_id, rest @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"coords tagOrId ?x1 y1 x2 y2?\"".to_string(),
    ));
  };

  let ids = resolve(canvas, tag_or_id)?;

  if rest.is_empty() {
    let Some(id) = ids.first() else {
      return Err(EvalError::ArgumentError(format!(
        "unknown canvas item: {}",
        tag_or_id.repr_str()?
      )));
    };
    let coords = canvas.item(*id).expect("resolved item").coords();
    return Ok(Value::from(coords.to_array().map(Value::from).to_vec()));
  }

  let [x1, y1, x2, y2] = rest else {
    return Err(EvalError::ArgumentError(
      "wrong number of coordinates: expected \"coords tagOrId ?x1 y1 x2 y2?\"".to_string(),
    ));
  };

  let coords = Rect {
    x1: x1.repr_float()?,
    y1: y1.repr_float()?,
    x2: x2.repr_float()?,
    y2: y2.repr_float()?,
  };
  for id in ids {
    canvas
      .item_mut(id)
      .expect("resolved item")
      .set_coords(coords);
  }

  Ok(Value::none())
}

fn eval_move(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  let [tag_or_id, dx, dy] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"move tagOrId xAmount yAmount\"".to_string(),
    ));
  };

  let ids = resolve(canvas, tag_or_id)?;
  let (dx, dy) = (dx.repr_float()?, dy.repr_float()?);

  for id in ids {
    let item = canvas.item_mut(id).expect("resolved item");
    let moved = item.coords().translated(dx, dy);
    item.set_coords(moved);
  }

  Ok(Value::none())
}

fn eval_delete(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  for tag_or_id in args.iter_mut() {
    for id in resolve(canvas, tag_or_id)? {
      canvas.delete(id);
    }
  }

  Ok(Value::none())
}

fn eval_itemconfigure(canvas: &mut Canvas, args: &mut [Value]) -> Result<Value, EvalError> {
  let [tag_or_id, options @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"itemconfigure tagOrId ?option value ...?\"".to_string(),
    ));
  };

  let ids = resolve(canvas, tag_or_id)?;

  for option in options.chunks_mut(2) {
    let [name, value] = option else {
      return Err(EvalError::ArgumentError(format!(
        "value for \"{}\" missing",
        option[0].repr_str()?
      )));
    };

    let name = name.repr_str()?.to_string();
    match name.as_str() {
      "-fill" => {
        let fill = optional_color(value)?;
        for id in &ids {
          canvas.item_mut(*id).expect("resolved item").set_fill(fill);
        }
      }
      _ => {
        return Err(EvalError::ArgumentError(format!(
          "unsupported item option: {}",
          name
        )));
      }
    }
  }

  Ok(Value::none())
}

/// Parses a color, where the empty string means "no color", as in Tk.
fn optional_color(value: &mut Value) -> Result<Option<Color>, EvalError> {
  let spec = value.repr_str()?;
  if spec.is_empty() {
    return Ok(None);
  }

  Color::parse(spec)
    .map(Some)
    .ok_or_else(|| EvalError::ArgumentError(format!("unknown color: {}", spec)))
}
