use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};
use std::{fmt::Write, mem::take};

#[derive(Default)]
struct FormatOpts {
  alt: bool,
  width: Option<usize>,
  pad: Option<FormatPad>,
  precision: Option<usize>,
}

enum FormatPad {
  Space,
  Zero,
}

enum NumFmt {
  DecSigned,
  DecUnsigned,
  Oct,
  HexLower,
  HexUpper,
  Bin,
  Char,
  Float,
}

pub(super) fn eval(
  args: &mut [Value],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let (format_str, rest) = match args {
    [format_str, rest @ ..] => (format_str, rest),
    _ => {
      return Err(EvalError::ArgumentError(
        "format expects at least one argument".to_string(),
      ));
    }
  };

  let format_str = format_str.repr_str();

  enum State {
    Copy,
    SubStart,
    Sub,
    SubPrec,
  }

  let mut state = State::Copy;
  let mut numeric_buffer: Option<usize> = None;
  let mut positional_arg = rest.iter();
  let mut selected_arg: Option<&Value> = None;
  let mut opts = FormatOpts::default();
  // heuristic initial capacity
  let mut out = String::with_capacity(format_str.len());

  macro_rules! build_number {
    ($ch:expr) => {
      let digit = $ch.to_digit(10).unwrap().saturating_cast::<usize>();
      numeric_buffer = match numeric_buffer {
        Some(n) => Some(n * 10 + digit),
        None => Some(digit),
      };
    };
  }

  macro_rules! w {
    ($ex:expr) => {{
      if let Some(n) = numeric_buffer {
        opts.width = Some(n);
      }
      state = State::Copy;
      $ex
    }};
  }

  macro_rules! arg {
    () => {
      match selected_arg {
        Some(arg) => Ok(arg),
        None => match positional_arg.next() {
          Some(arg) => Ok(arg),
          None => Err(EvalError::ArgumentError(
            "not enough template arguments".to_string(),
          )),
        },
      }
    };
  }

  let mut chars = format_str.chars().peekable();
  while let Some(&ch) = chars.peek() {
    match state {
      State::Copy => {
        chars.next();
        if ch == '%' {
          numeric_buffer = None;
          selected_arg = None;
          opts = FormatOpts::default();
          state = State::SubStart;
        } else {
          out.push(ch);
        }
      }
      State::SubStart => match ch {
        '#' => {
          chars.next();
          opts.alt = true;
        }
        '0' => {
          chars.next();
          opts.pad = Some(FormatPad::Zero);
        }
        _ => {
          state = State::Sub;
        }
      },
      State::Sub => {
        chars.next();
        match ch {
          ch @ '0'..='9' => {
            build_number!(ch);
          }
          '%' => {
            out.push('%');
            state = State::Copy;
          }
          '$' => {
            let Some(index) = take(&mut numeric_buffer) else {
              return Err(EvalError::ArgumentError("bad field specifier".to_string()));
            };
            let Some(selected) = rest.get(index) else {
              return Err(EvalError::ArgumentError(format!(
                "%{index}$ argument index out of range"
              )));
            };
            selected_arg = Some(selected);
          }
          '.' => {
            opts.width = take(&mut numeric_buffer);
            state = State::SubPrec;
          }
          'd' | 'i' => w!(write_number(&mut out, NumFmt::DecSigned, &opts, arg!()?)?),
          'u' => w!(write_number(&mut out, NumFmt::DecUnsigned, &opts, arg!()?)?),
          'x' => w!(write_number(&mut out, NumFmt::HexLower, &opts, arg!()?)?),
          'X' => w!(write_number(&mut out, NumFmt::HexUpper, &opts, arg!()?)?),
          'o' => w!(write_number(&mut out, NumFmt::Oct, &opts, arg!()?)?),
          'b' => w!(write_number(&mut out, NumFmt::Bin, &opts, arg!()?)?),
          'c' => w!(write_number(&mut out, NumFmt::Char, &opts, arg!()?)?),
          'f' => w!(write_number(&mut out, NumFmt::Float, &opts, arg!()?)?),
          's' => w!(write!(out, "{}", arg!()?.repr_str()).unwrap()),
          _ => {
            return Err(EvalError::ArgumentError(format!(
              "unsupported format specifier: {ch}"
            )));
          }
        }
      }
      State::SubPrec => match ch {
        ch @ '0'..='9' => {
          chars.next();
          build_number!(ch);
        }
        _ => {
          opts.precision = Some(take(&mut numeric_buffer).unwrap_or(0));
          state = State::Sub;
        }
      },
    }
  }

  if !matches!(state, State::Copy) {
    return Err(EvalError::ArgumentError(
      "format string ended in the middle of field specifier".to_string(),
    ));
  }

  Ok(Value::from(out))
}

fn write_number(
  out: &mut String,
  format: NumFmt,
  opts: &FormatOpts,
  value: &Value,
) -> Result<(), EvalError> {
  use NumFmt::*;
  match format {
    DecSigned => write_num_pad(out, format, opts, |out| {
      write!(out, "{}", value.repr_int()? as i32).unwrap();
      Ok(())
    }),
    DecUnsigned => write_num_pad(out, format, opts, |out| {
      write!(out, "{}", value.repr_int()? as u32).unwrap();
      Ok(())
    }),
    Oct => write_num_pad(out, format, opts, |out| {
      write!(out, "{:o}", value.repr_int()? as u32).unwrap();
      Ok(())
    }),
    HexLower => write_num_pad(out, format, opts, |out| {
      write!(out, "{:x}", value.repr_int()? as u32).unwrap();
      Ok(())
    }),
    HexUpper => write_num_pad(out, format, opts, |out| {
      write!(out, "{:X}", value.repr_int()? as u32).unwrap();
      Ok(())
    }),
    Bin => write_num_pad(out, format, opts, |out| {
      write!(out, "{:b}", value.repr_int()? as u32).unwrap();
      Ok(())
    }),
    Char => write_num_pad(out, format, opts, |out| {
      write!(
        out,
        "{}",
        value
          .repr_int()?
          .checked_cast::<u32>()
          .and_then(|u| char::from_u32(u))
          .ok_or_else(|| EvalError::ArgumentError(
            "value too large to convert to char".to_string()
          ))?
      )
      .unwrap();
      Ok(())
    }),
    Float => write_num_pad(out, format, opts, |out| {
      write!(
        out,
        "{:.precision$}",
        value.repr_float()? as f32,
        precision = opts.precision.unwrap_or(6)
      )
      .unwrap();
      Ok(())
    }),
  }
}

fn write_num_pad(
  out: &mut String,
  format: NumFmt,
  opts: &FormatOpts,
  write_body: impl FnOnce(&mut String) -> Result<(), EvalError>,
) -> Result<(), EvalError> {
  use NumFmt::*;

  let output_start = out.len();
  if opts.alt {
    let prefix = match format {
      DecSigned | DecUnsigned => "0d",
      Oct => "0o",
      HexLower | HexUpper => "0x",
      Bin => "0b",
      Char | Float => "",
    };
    write!(out, "{}", prefix).unwrap();
  }

  let body_start = out.len();
  write_body(out)?;

  let len = out.len() - output_start;
  if let Some(width) = opts.width {
    let (pad_str, pad_index) = match opts.pad.as_ref().unwrap_or(&FormatPad::Space) {
      FormatPad::Space => (" ", output_start),
      FormatPad::Zero => ("0", body_start),
    };
    out.insert_str(pad_index, pad_str.repeat(0.max(width - len)).as_str());
  }

  Ok(())
}
