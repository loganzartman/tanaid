use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::value::Value;
use std::collections::HashMap;

struct VwaitOptions<'a> {
  vars: Vec<&'a str>,
}

pub(super) async fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  frame: FrameId,
) -> EvalCmdResult {
  let opts = match args {
    [var] => VwaitOptions {
      vars: vec![var.repr_str()?],
    },
    _ => todo!("vwait with options not supported"),
  };

  eval_vwait(opts, context, frame).await
}

async fn eval_vwait<'a>(
  opts: VwaitOptions<'a>,
  context: &mut EvalContext,
  frame: FrameId,
) -> EvalCmdResult {
  let mut values: HashMap<String, Value> = HashMap::new();
  for var in opts.vars.iter() {
    values.insert(
      var.to_string(),
      context
        .get_variable(frame, var)
        .cloned()
        .unwrap_or(Value::none()),
    );
  }

  'outer: loop {
    let Some(delay) = context.next_event_delay() else {
      break;
    };

    context.sleep_ms(delay.as_millis() as u64).await?;
    context.poll_event().await?;

    for var in opts.vars.iter() {
      let mut old_val = values.get(*var).cloned().unwrap_or(Value::none());
      let mut new_val = context
        .get_variable(frame, var)
        .cloned()
        .unwrap_or(Value::none());
      if new_val.ne(&mut old_val)?.repr_bool()? {
        break 'outer;
      }
    }
  }

  Ok(Value::none())
}
