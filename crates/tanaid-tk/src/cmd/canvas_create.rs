use crate::canvas::CanvasWidget;
use crate::item::canvas_item::CanvasItem;
use crate::item::line::CanvasItemLine;
use crate::item::rect::CanvasItemRect;
use crate::tk_context::TkContext;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub(crate) fn eval(
  args: &mut [Value],
  _ctx: &mut EvalContext,
  _frame: FrameId,
  _tk: &TkContext,
  widget: &mut CanvasWidget,
) -> EvalCmdResult {
  let (item_type, rest) = match args {
    [item_type, rest @ ..] => (item_type.repr_str()?, rest),
    _ => {
      return Err(EvalError::ArgumentError(
        "wrong number of args; should be: pathName create itemType ?option ...?".to_string(),
      ));
    }
  };

  match item_type {
    "rectangle" => create_rectangle(rest, widget),
    "line" => create_line(rest, widget),
    _ => Err(EvalError::ArgumentError(format!(
      "invalid item type: {}",
      item_type
    ))),
  }
}

fn next_id(widget: &CanvasWidget) -> i64 {
  widget.items.last().map_or(0, |(id, _)| id + 1)
}

fn insert_item(widget: &mut CanvasWidget, item: impl CanvasItem + 'static) -> EvalCmdResult {
  let id = next_id(widget);
  widget.items.insert(id, Box::new(item));

  Ok(Value::from(id))
}

fn is_option_name(value: &mut Value) -> bool {
  // optimization: will never match RE_OPT; doesn't materialize string
  if value.peek_number().is_some() {
    return false;
  }

  match value.repr_str() {
    Ok(val) => matches!(val.as_bytes(), [b'-', b'a'..=b'z', ..]),
    Err(_) => false,
  }
}

fn parse_coords_and_options<'a>(
  name: &str,
  opts: &'a mut [Value],
) -> Result<(Vec<f64>, &'a mut [[Value; 2]]), EvalError> {
  let options_index = opts
    .iter_mut()
    .position(|x| is_option_name(x))
    .unwrap_or(opts.len());
  let (coords, rest) = opts.split_at_mut(options_index);
  let (options, rest) = rest.as_chunks_mut::<2>();

  let coords = if coords.len() == 1 {
    coords[0]
      .repr_list()?
      .iter()
      .map(|x| x.clone().repr_float())
      .collect::<Result<Vec<_>, _>>()?
  } else {
    coords
      .iter_mut()
      .map(|x| x.repr_float())
      .collect::<Result<Vec<_>, _>>()?
  };

  if coords.is_empty() {
    return Err(EvalError::ArgumentError(format!(
      "wrong number of args; should be: pathName create {name} x y ?x2 y2...? ?option ...?"
    )));
  }

  if let Some(extra) = rest.last() {
    return Err(EvalError::ArgumentError(format!(
      "{name}: missing value for {}",
      extra.to_string()
    )));
  }

  Ok((coords, options))
}

fn apply_options(item: &mut impl CanvasItem, options: &mut [[Value; 2]]) -> Result<(), EvalError> {
  for [option, value] in options {
    let name = option.repr_str()?;
    if item.apply_own_option(name, value)? {
      continue;
    }
    if item.apply_common_option(name, value)? {
      continue;
    }
    return Err(EvalError::ArgumentError(format!(
      "unsupported option: {name} = {value}"
    )));
  }
  Ok(())
}

fn create_rectangle(opts: &mut [Value], widget: &mut CanvasWidget) -> EvalCmdResult {
  let (coords, options) = parse_coords_and_options("rectangle", opts)?;
  let mut item = CanvasItemRect::default().with_coords(&coords)?;
  apply_options(&mut item, options)?;
  insert_item(widget, item)
}

fn create_line(opts: &mut [Value], widget: &mut CanvasWidget) -> EvalCmdResult {
  let (coords, options) = parse_coords_and_options("line", opts)?;
  let mut item = CanvasItemLine::default().with_coords(&coords)?;
  apply_options(&mut item, options)?;
  insert_item(widget, item)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// parse args as `create line` would, returning coords and option names
  fn parse(args: &[&str]) -> Result<(Vec<f64>, Vec<String>), EvalError> {
    let mut args: Vec<Value> = args.iter().map(|s| Value::from(*s)).collect();
    let (coords, options) = parse_coords_and_options("line", &mut args)?;
    let names = options.iter().map(|[name, _]| name.to_string()).collect();
    Ok((coords, names))
  }

  #[test]
  fn splits_coords_and_options() {
    let (coords, names) = parse(&["0", "0", "10", "10", "-fill", "red", "-width", "2"]).unwrap();
    assert_eq!(coords, [0., 0., 10., 10.]);
    assert_eq!(names, ["-fill", "-width"]);
  }

  #[test]
  fn negative_coords_are_not_options() {
    let (coords, names) = parse(&["-5", "-5.5", "10", "-Inf"]).unwrap();
    assert_eq!(coords, [-5., -5.5, 10., f64::NEG_INFINITY]);
    assert!(names.is_empty());
  }

  #[test]
  fn coords_as_single_list() {
    let (coords, names) = parse(&["0 0 10 10", "-fill", "red"]).unwrap();
    assert_eq!(coords, [0., 0., 10., 10.]);
    assert_eq!(names, ["-fill"]);
  }

  #[test]
  fn rejects_missing_option_value() {
    assert!(parse(&["0", "0", "10", "10", "-fill"]).is_err());
  }

  #[test]
  fn rejects_bad_coord() {
    assert!(parse(&["0", "0", "10", "1O"]).is_err());
  }

  #[test]
  fn rejects_no_coords() {
    assert!(parse(&["-fill", "red"]).is_err());
  }
}
