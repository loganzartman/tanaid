use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval_error::EvalError, value::Value};

enum Tasks {
  All,
  Idle,
}

pub(super) async fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  match args {
    [] => eval_update(context, Tasks::All).await,
    [arg] if arg.to_string() == "idle" => eval_update(context, Tasks::Idle).await,
    _ => Err(EvalError::ArgumentError(
      "wrong arguments, expects: update ?idletasks?".to_string(),
    )),
  }
}

async fn eval_update(context: &mut EvalContext, tasks: Tasks) -> EvalCmdResult {
  match tasks {
    Tasks::All => {}
    _ => return Err(EvalError::NotImplemented),
  }

  while context.count_pending_events() > 0 {
    let Some(delay) = context.next_event_delay() else {
      break;
    };

    context.sleep_ms(delay.as_millis() as u64).await?;
    context.poll_event().await?;
  }

  Ok(Value::none())
}
