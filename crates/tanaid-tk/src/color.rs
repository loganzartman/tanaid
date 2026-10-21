use tanaid::eval_error::EvalError;

// RGBA [0, 1]
pub type Color = (f32, f32, f32, f32);

pub fn parse_color(str: &str) -> Result<Color, EvalError> {
  if str.starts_with('#') {
    return parse_color_hex(str);
  }
  Err(EvalError::ArgumentError(
    "only hex colors are supported".to_string(),
  ))
}

pub fn parse_color_hex(str: &str) -> Result<Color, EvalError> {
  let Some(str) = str.strip_prefix('#') else {
    return Err(EvalError::ArgumentError(
      "hex color must begin with #".to_string(),
    ));
  };

  match str.len() {
    3 => parse_color_hex_n::<1>(str),
    6 => parse_color_hex_n::<2>(str),
    9 => parse_color_hex_n::<3>(str),
    12 => parse_color_hex_n::<4>(str),
    _ => Err(EvalError::ArgumentError(
      "invalid hex color; expects: #RGB, #RRGGBB, #RRRGGGBBB, or #RRRRGGGGBBBB".to_string(),
    )),
  }
}

fn parse_color_hex_n<const N: usize>(str: &str) -> Result<Color, EvalError> {
  let scale = 1. / 16_f32.powf(N as f32);

  let parse = |s: &str| -> Result<f32, EvalError> {
    let val_int = u16::from_str_radix(s, 16)
      .map_err(|e| EvalError::Generic(format!("error parsing hex component {str}: {e}")))?;
    Ok(val_int as f32 * scale)
  };

  let r = parse(&str[0..N])?;
  let g = parse(&str[N..N * 2])?;
  let b = parse(&str[N * 2..N * 3])?;
  Ok((r, g, b, 1.0))
}
