use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{
  eval_error::EvalError,
  value::{Int, Value},
};
use std::{fmt::Write, mem::take};

#[derive(Default)]
struct FormatOpts {
  alt: bool,
  width: Option<usize>,
  pad_zero: bool,
  left_align: bool,
  precision: Option<usize>,
  size: ArgSize,
}

#[derive(Default)]
enum ArgSize {
  X16,
  #[default]
  X32,
  X64,
  NoTrunc,
}

impl ArgSize {
  fn signed(&self, n: Int) -> i64 {
    match self {
      ArgSize::X16 => n as i16 as i64,
      ArgSize::X32 => n as i32 as i64,
      ArgSize::X64 | ArgSize::NoTrunc => n,
    }
  }

  fn unsigned(&self, n: Int) -> (bool, u64) {
    match self {
      ArgSize::X16 => (false, n as u16 as u64),
      ArgSize::X32 => (false, n as u32 as u64),
      ArgSize::X64 => (false, n as u64),
      ArgSize::NoTrunc => (n < 0, n.unsigned_abs()),
    }
  }
}

#[derive(Clone, Copy)]
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
    SubSize,
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
          opts.pad_zero = true;
        }
        '-' => {
          chars.next();
          opts.left_align = true;
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
          state = State::SubSize;
        }
      },
      State::SubPrec => match ch {
        ch @ '0'..='9' => {
          chars.next();
          build_number!(ch);
        }
        _ => {
          opts.precision = Some(take(&mut numeric_buffer).unwrap_or(0));
          state = State::SubSize;
        }
      },
      State::SubSize => match ch {
        'h' => {
          chars.next();
          opts.size = ArgSize::X16;
          state = State::SubType;
        }
        'l' => {
          chars.next();
          if chars.peek() == Some(&'l') {
            chars.next();
            opts.size = ArgSize::NoTrunc;
          } else {
            opts.size = ArgSize::X64;
          }
          state = State::SubType;
        }
        'z' | 't' | 'j' | 'q' => {
          chars.next();
          opts.size = ArgSize::X64;
          state = State::SubType;
        }
        'L' => {
          chars.next();
          opts.size = ArgSize::NoTrunc;
          state = State::SubType;
        }
        _ => {
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
      write!(out, "{}", opts.size.signed(value.repr_int()?)).unwrap();
      Ok(())
    }),
    DecUnsigned | Oct | HexLower | HexUpper | Bin => write_arg_pad(out, format, opts, |out| {
      let (negative, n) = opts.size.unsigned(value.repr_int()?);
      if negative {
        out.push('-');
      }
      match format {
        DecUnsigned => write!(out, "{n}"),
        Oct => write!(out, "{n:o}"),
        HexLower => write!(out, "{n:x}"),
        HexUpper => write!(out, "{n:X}"),
        Bin => write!(out, "{n:b}"),
        _ => unreachable!(),
      }
      .unwrap();
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
      let mut str = value.repr_str();
      if let Some(precision) = opts.precision {
        str = &str[0..str
          .char_indices()
          .nth(precision)
          .map(|x| x.0)
          .unwrap_or(str.len())];
      }
      write!(out, "{}", str).unwrap();
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
  write_body(out)?;

  let mut body_start = output_start;
  if out[body_start..].starts_with('-') {
    body_start += 1;
  }

  if opts.alt {
    let prefix = match format {
      DecSigned | DecUnsigned => "0d",
      Oct => "0o",
      HexLower | HexUpper => "0x",
      Bin => "0b",
      Char | Float | String => "",
    };
    out.insert_str(body_start, prefix);
    body_start += prefix.len();
  }

  let len = out[output_start..].chars().count();
  if let Some(width) = opts.width {
    let (pad_str, pad_index) = match (opts.pad_zero, opts.left_align) {
      (false, false) => (" ", output_start),
      (false, true) => (" ", out.len()),
      (true, _) => ("0", body_start),
    };
    out.insert_str(
      pad_index,
      pad_str.repeat(width.saturating_sub(len)).as_str(),
    );
  }

  Ok(())
}
