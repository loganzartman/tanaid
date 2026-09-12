use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::{eval::context::GLOBAL_FRAME, value::Value};
use std::collections::HashMap;

struct VwaitOptions<'a> {
  vars: Vec<&'a str>,
}

pub(super) async fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let opts = match args {
    [var] => VwaitOptions {
      vars: vec![var.repr_str()?],
    },
    _ => todo!("vwait with options not supported"),
  };

  eval_vwait(opts, context).await
}

async fn eval_vwait<'a>(opts: VwaitOptions<'a>, context: &mut EvalContext) -> EvalCmdResult {
  let mut revs: HashMap<String, u64> = HashMap::new();
  for var in opts.vars.iter() {
    revs.insert(var.to_string(), context.get_variable_rev(GLOBAL_FRAME, var));
  }

  'outer: loop {
    // TODO: wait for future events when no timers pending
    let Some(delay) = context.next_event_delay() else {
      break;
    };

    context.sleep_ms(delay.as_millis() as u64).await?;
    context.poll_event().await?;

    for var in opts.vars.iter() {
      let old_rev = *revs.get(*var).unwrap_or(&0);
      let new_rev = context.get_variable_rev(GLOBAL_FRAME, var);
      if new_rev != old_rev {
        break 'outer;
      }
    }
  }

  Ok(Value::none())
}
