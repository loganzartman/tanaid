use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::value::Value;

struct VwaitOptions<'a> {
  vars: Vec<&'a str>,
}

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let opts = match args {
    [var] => VwaitOptions {
      vars: vec![var.repr_str()?],
    },
    _ => todo!("vwait with options not supported"),
  };

  eval_vwait(opts, context, frame)
}

// 1. record the value of all vars
// 2. run the event loop
// 2a. take next event
// 2b. check variables
// 2c. if changed, resume.
// 2d. else, continue 2a.
fn eval_vwait(opts: VwaitOptions, _context: &mut EvalContext, _frame: FrameId) -> EvalCmdResult {
  println!("vwait {}", opts.vars.join(" "));
  Ok(Value::none())
}
