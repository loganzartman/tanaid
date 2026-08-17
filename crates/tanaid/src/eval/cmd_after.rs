use std::{thread::sleep, time::Duration};

use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, parser, value::Value};

pub(super) fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  match args {
    [subcommand, rest @ ..] if subcommand.to_string() == "cancel" => eval_cancel(rest, context),
    [subcommand, rest @ ..] if subcommand.to_string() == "idle" => eval_idle(rest, context),
    [ms, rest @ ..] => eval_ms(ms, rest, context),
    _ => Err(EvalError::ArgumentError(
      "wrong number of args: expected \"after option ?arg ...?\"".to_string(),
    )),
  }
}

fn eval_cancel(args: &mut [Value], context: &mut EvalContext) -> EvalCmdResult {
  let id = match args {
    [id] => id.repr_int()?,
    _ => unimplemented!("only supports \"after cancel id\""),
  };
  context
    .cancel_timer(
      id.try_into()
        .map_err(|e| EvalError::ArgumentError(format!("failed to convert timer id: {}", e)))?,
    )
    .map(|_| Value::none())
}

fn eval_idle(_args: &mut [Value], _context: &mut EvalContext) -> EvalCmdResult {
  todo!()
}

fn eval_ms(ms: &mut Value, args: &mut [Value], context: &mut EvalContext) -> EvalCmdResult {
  let delay_ms: u64 = ms
    .repr_int()?
    .try_into()
    .map_err(|_| EvalError::ArgumentError("\"after\" duration must be positive".to_string()))?;

  if args.is_empty() {
    sleep(Duration::from_millis(delay_ms));
    return Ok(Value::none());
  }

  let script_src: String = args
    .iter_mut()
    .map(|arg| arg.repr_str())
    .collect::<Result<Vec<_>, _>>()?
    .join(" ");

  let timer_script = parser::parse(script_src.as_str())
    .map_err(|e| EvalError::ScriptParseError(format!("failed to parse script: {}", e)))?;

  Ok(Value::from(
    // TODO: cast
    context.start_timer(&timer_script, delay_ms)? as i64,
  ))
}
