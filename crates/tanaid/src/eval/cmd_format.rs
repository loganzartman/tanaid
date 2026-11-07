use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};
use std::{fmt::Write, mem::take};

struct FormatOpts {
  width: Option<u32>,
}

impl FormatOpts {
  fn new() -> Self {
    FormatOpts { width: None }
  }
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
        "unsupported clock subcommand".to_string(),
      ));
    }
  };

  let format_str = format_str.repr_str();

  enum State {
    Copy,
    Sub,
  }

  let mut state = State::Copy;
  let mut numeric_buffer: Option<u32> = None;
  let mut positional_arg = rest.iter();
  let mut selected_arg: Option<&Value> = None;
  let mut opts = FormatOpts::new();
  // heuristic initial capacity
  let mut out = String::with_capacity(format_str.len());

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

  for ch in format_str.chars() {
    match state {
      State::Copy => {
        if ch == '%' {
          numeric_buffer = None;
          selected_arg = None;
          state = State::Sub;
        } else {
          out.push(ch);
        }
      }
      State::Sub => match ch {
        ch @ '0'..'9' => {
          let digit = ch.to_digit(10).unwrap();
          numeric_buffer = match numeric_buffer {
            Some(n) => Some(n * 10 + digit),
            None => Some(digit),
          };
        }
        '$' => {
          let Some(index) = take(&mut numeric_buffer) else {
            return Err(EvalError::ArgumentError("bad field specifier".to_string()));
          };
          let Some(selected) = rest.get(index.saturating_cast::<usize>()) else {
            return Err(EvalError::ArgumentError(format!(
              "%{index}$ argument index out of range"
            )));
          };
          selected_arg = Some(selected);
        }
        'd' => do_format!(format_int(&mut out, arg!()?, &opts)?),
        _ => {
          return Err(EvalError::ArgumentError(format!(
            "unknown format specifier: {ch}"
          )));
        }
      },
    }
  }

  Ok(Value::from(out))
}

fn format_int(out: &mut String, arg: &Value, opts: &FormatOpts) -> Result<(), EvalError> {
  write!(
    out,
    "{:width$}",
    arg.repr_int()?,
    width = opts.width.map(|w| w.saturating_cast()).unwrap_or(0)
  )
  .map_err(|e| EvalError::Generic(format!("write error: {}", e.to_string())))
}
