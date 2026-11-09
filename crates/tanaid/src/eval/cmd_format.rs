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

enum ArgFmt {
  DecSigned,
  DecUnsigned,
  Oct,
  HexLower,
  HexUpper,
  Bin,
  Char,
  Float,
  String,
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
    SubPos,
    SubFlag,
    SubWidth,
    SubPrec,
    SubType,
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
        Some(n) => Some(
          n.checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| EvalError::ArgumentError("padding too large".to_string()))?,
        ),
        None => Some(digit),
      };
    };
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
      State::Copy => match ch {
        '%' => {
          chars.next();
          numeric_buffer = None;
          selected_arg = None;
          opts = FormatOpts::default();
          state = State::SubPos;
        }
        _ => {
          chars.next();
          out.push(ch);
        }
      },
      State::SubPos => match ch {
        '%' => {
          chars.next();
          out.push('%');
          state = State::Copy;
        }
        ch @ '0'..='9' if ch != '0' || numeric_buffer.is_some() => {
          chars.next();
          build_number!(ch);
        }
        '$' => {
          chars.next();
          let Some(index) = take(&mut numeric_buffer) else {
            return Err(EvalError::ArgumentError("bad field specifier".to_string()));
          };
          let Some(selected) = index.checked_sub(1).and_then(|i| rest.get(i)) else {
            return Err(EvalError::ArgumentError(format!(
              "%{index}$ argument index out of range"
            )));
          };
          selected_arg = Some(selected);
          state = State::SubFlag;
        }
        _ => {
          state = State::SubFlag;
        }
      },
      State::SubFlag => match ch {
        '#' => {
          chars.next();
          opts.alt = true;
        }
        '0' => {
          chars.next();
          opts.pad = Some(FormatPad::Zero);
        }
        _ => {
          state = State::SubWidth;
        }
      },
      State::SubWidth => match ch {
        ch @ '0'..='9' => {
          chars.next();
          build_number!(ch);
        }
        '.' => {
          chars.next();
          opts.width = take(&mut numeric_buffer);
          state = State::SubPrec;
        }
        _ => {
          opts.width = take(&mut numeric_buffer);
          state = State::SubType;
        }
      },
      State::SubPrec => match ch {
        ch @ '0'..='9' => {
          chars.next();
          build_number!(ch);
        }
        _ => {
          opts.precision = Some(take(&mut numeric_buffer).unwrap_or(0));
          state = State::SubType;
        }
      },
      State::SubType => {
        chars.next();
        match ch {
          'd' | 'i' => write_arg(&mut out, ArgFmt::DecSigned, &opts, arg!()?)?,
          'u' => write_arg(&mut out, ArgFmt::DecUnsigned, &opts, arg!()?)?,
          'x' => write_arg(&mut out, ArgFmt::HexLower, &opts, arg!()?)?,
          'X' => write_arg(&mut out, ArgFmt::HexUpper, &opts, arg!()?)?,
          'o' => write_arg(&mut out, ArgFmt::Oct, &opts, arg!()?)?,
          'b' => write_arg(&mut out, ArgFmt::Bin, &opts, arg!()?)?,
          'c' => write_arg(&mut out, ArgFmt::Char, &opts, arg!()?)?,
          'f' => write_arg(&mut out, ArgFmt::Float, &opts, arg!()?)?,
          's' => write_arg(&mut out, ArgFmt::String, &opts, arg!()?)?,
          _ => {
            return Err(EvalError::ArgumentError(format!(
              "unsupported format specifier: {ch}"
            )));
          }
        }
        state = State::Copy;
      }
    }
  }

  if !matches!(state, State::Copy) {
    return Err(EvalError::ArgumentError(
      "format string ended in the middle of field specifier".to_string(),
    ));
  }

  Ok(Value::from(out))
}

fn write_arg(
  out: &mut String,
  format: ArgFmt,
  opts: &FormatOpts,
  value: &Value,
) -> Result<(), EvalError> {
  use ArgFmt::*;
  match format {
    DecSigned => write_arg_pad(out, format, opts, |out| {
      write!(out, "{}", value.repr_int()? as i64).unwrap();
      Ok(())
    }),
    DecUnsigned => write_arg_pad(out, format, opts, |out| {
      write!(out, "{}", value.repr_int()? as u64).unwrap();
      Ok(())
    }),
    Oct => write_arg_pad(out, format, opts, |out| {
      write!(out, "{:o}", value.repr_int()? as u64).unwrap();
      Ok(())
    }),
    HexLower => write_arg_pad(out, format, opts, |out| {
      write!(out, "{:x}", value.repr_int()? as u64).unwrap();
      Ok(())
    }),
    HexUpper => write_arg_pad(out, format, opts, |out| {
      write!(out, "{:X}", value.repr_int()? as u64).unwrap();
      Ok(())
    }),
    Bin => write_arg_pad(out, format, opts, |out| {
      write!(out, "{:b}", value.repr_int()? as u64).unwrap();
      Ok(())
    }),
    Char => write_arg_pad(out, format, opts, |out| {
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
    Float => write_arg_pad(out, format, opts, |out| {
      write!(
        out,
        "{:.precision$}",
        value.repr_float()?,
        precision = opts.precision.unwrap_or(6)
      )
      .unwrap();
      Ok(())
    }),
    String => write_arg_pad(out, format, opts, |out| {
      write!(out, "{}", value.repr_str()).unwrap();
      Ok(())
    }),
  }
}

fn write_arg_pad(
  out: &mut String,
  format: ArgFmt,
  opts: &FormatOpts,
  write_body: impl FnOnce(&mut String) -> Result<(), EvalError>,
) -> Result<(), EvalError> {
  use ArgFmt::*;

  let output_start = out.len();
  if opts.alt {
    let prefix = match format {
      DecSigned | DecUnsigned => "0d",
      Oct => "0",
      HexLower => "0x",
      HexUpper => "0X",
      Bin => "0b",
      Char | Float | String => "",
    };
    write!(out, "{}", prefix).unwrap();
  }

  let mut body_start = out.len();
  write_body(out)?;

  if out[body_start..].starts_with('-') {
    body_start += 1;
  }

  let len = out[output_start..].chars().count();
  if let Some(width) = opts.width {
    let (pad_str, pad_index) = match opts.pad.as_ref().unwrap_or(&FormatPad::Space) {
      FormatPad::Space => (" ", output_start),
      FormatPad::Zero => ("0", body_start),
    };
    out.insert_str(
      pad_index,
      pad_str.repeat(width.saturating_sub(len)).as_str(),
    );
  }

  Ok(())
}
