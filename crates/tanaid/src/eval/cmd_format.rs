use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};
use std::{fmt::Write, mem::take};

#[derive(Default)]
struct FormatOpts {
  width: Option<usize>,
  precision: Option<usize>,
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

  macro_rules! do_format {
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
          state = State::Sub;
        } else {
          out.push(ch);
        }
      }
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
          'd' | 'i' => do_format!(format_int(&mut out, arg!()?, &opts)?),
          'f' => do_format!(format_float(&mut out, arg!()?, &opts)?),
          _ => {
            return Err(EvalError::ArgumentError(format!(
              "unknown format specifier: {ch}"
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

fn format_int(out: &mut String, arg: &Value, opts: &FormatOpts) -> Result<(), EvalError> {
  write!(
    out,
    "{:width$}",
    arg.repr_int()?,
    width = opts.width.unwrap_or(0)
  )
  .map_err(|e| EvalError::Generic(format!("write error: {}", e.to_string())))
}

fn format_float(out: &mut String, arg: &Value, opts: &FormatOpts) -> Result<(), EvalError> {
  match opts.precision {
    Some(precision) => {
      write!(
        out,
        "{:width$.precision$}",
        arg.repr_float()?,
        width = opts.width.map(|w| w.saturating_cast()).unwrap_or(0),
        precision = precision,
      )
    }
    None => {
      write!(
        out,
        "{:width$}",
        arg.repr_float()?,
        width = opts.width.map(|w| w.saturating_cast()).unwrap_or(0),
      )
    }
  }
  .map_err(|e| EvalError::Generic(format!("write error: {}", e.to_string())))
}
