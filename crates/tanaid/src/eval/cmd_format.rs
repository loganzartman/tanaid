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
  fn signed(&self, n: Int) -> (bool, u64) {
    let n = match self {
      ArgSize::X16 => n as i16 as i64,
      ArgSize::X32 => n as i32 as i64,
      ArgSize::X64 | ArgSize::NoTrunc => n,
    };
    (n < 0, n.unsigned_abs())
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
  Int(IntFmt),
  Char,
  Float,
  String,
}

#[derive(Clone, Copy)]
enum IntFmt {
  DecSigned,
  DecUnsigned,
  Oct,
  HexLower,
  HexUpper,
  Bin,
}

struct FormattedArg {
  sign: &'static str,
  prefix: &'static str,
  body: String,
  should_zero_pad: bool,
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
        let format = match ch {
          'd' | 'i' => ArgFmt::Int(IntFmt::DecSigned),
          'u' => ArgFmt::Int(IntFmt::DecUnsigned),
          'x' => ArgFmt::Int(IntFmt::HexLower),
          'X' => ArgFmt::Int(IntFmt::HexUpper),
          'o' => ArgFmt::Int(IntFmt::Oct),
          'b' => ArgFmt::Int(IntFmt::Bin),
          'c' => ArgFmt::Char,
          'f' => ArgFmt::Float,
          's' => ArgFmt::String,
          _ => {
            return Err(EvalError::ArgumentError(format!(
              "unsupported format specifier: {ch}"
            )));
          }
        };
        let formatted = format_arg(format, &opts, arg!()?)?;
        write_arg(&mut out, formatted, &opts);
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

fn format_arg(format: ArgFmt, opts: &FormatOpts, value: &Value) -> Result<FormattedArg, EvalError> {
  Ok(match format {
    ArgFmt::Char => FormattedArg {
      sign: "",
      prefix: "",
      body: value
        .repr_int()?
        .checked_cast::<u32>()
        .and_then(|u| char::from_u32(u))
        .ok_or_else(|| EvalError::ArgumentError("value too large to convert to char".to_string()))?
        .to_string(),
      should_zero_pad: true,
    },
    ArgFmt::Float => {
      let f = value.repr_float()?;
      FormattedArg {
        sign: if f.is_sign_negative() && !f.is_nan() {
          "-"
        } else {
          ""
        },
        prefix: "",
        body: format!(
          "{:.precision$}",
          f.abs(),
          precision = opts.precision.unwrap_or(6)
        ),
        should_zero_pad: f.is_finite(),
      }
    }
    ArgFmt::Int(int_fmt) => {
      let (negative, n) = match int_fmt {
        IntFmt::DecSigned => opts.size.signed(value.repr_int()?),
        _ => opts.size.unsigned(value.repr_int()?),
      };
      let (prefix, numeric) = match int_fmt {
        IntFmt::Bin => ("0b", format!("{n:b}")),
        IntFmt::DecSigned | IntFmt::DecUnsigned => ("0d", format!("{n}")),
        IntFmt::HexLower => ("0x", format!("{n:x}")),
        IntFmt::HexUpper => ("0x", format!("{n:X}")),
        IntFmt::Oct => ("0o", format!("{n:o}")),
      };
      FormattedArg {
        sign: if negative { "-" } else { "" },
        prefix: if opts.alt { prefix } else { "" },
        body: format!(
          "{numeric:0>precision$}",
          precision = opts.precision.unwrap_or(0)
        ),
        should_zero_pad: opts.precision.is_none(),
      }
    }
    ArgFmt::String => {
      let mut str = value.repr_str();
      if let Some(precision) = opts.precision {
        str = &str[0..str
          .char_indices()
          .nth(precision)
          .map(|x| x.0)
          .unwrap_or(str.len())];
      }

      FormattedArg {
        sign: "",
        prefix: "",
        body: str.to_string(),
        should_zero_pad: true,
      }
    }
  })
}

fn write_arg(out: &mut String, formatted: FormattedArg, opts: &FormatOpts) {
  let FormattedArg {
    sign,
    prefix,
    body,
    should_zero_pad,
  } = formatted;
  let len = sign.len() + prefix.len() + body.chars().count();
  let pad_count = opts.width.unwrap_or(0).saturating_sub(len);
  let (leftpad, zeropad, rightpad) = match (opts.pad_zero && should_zero_pad, opts.left_align) {
    (true, _) => (0, pad_count, 0),
    (false, true) => (0, 0, pad_count),
    (false, false) => (pad_count, 0, 0),
  };
  write!(
    out,
    "{:leftpad$}{sign}{prefix}{:0<zeropad$}{body}{:rightpad$}",
    "", "", ""
  )
  .unwrap();
}
