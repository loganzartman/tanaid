use super::{
  EvalContext, FrameId, cmd_break, cmd_continue, cmd_dict, cmd_expr, cmd_foreach, cmd_global,
  cmd_if, cmd_incr, cmd_info, cmd_lappend, cmd_lindex, cmd_list, cmd_llength, cmd_lreverse,
  cmd_proc, cmd_puts, cmd_return, cmd_set, cmd_string, cmd_uplevel, cmd_upvar, cmd_while,
};
use crate::eval::cmd_unknown;
use crate::eval_error::EvalError;
use crate::value::Value;

pub(super) type EvalCmdResult = Result<Value, EvalError>;
type EvalCmd = fn(&mut [Value], &mut EvalContext, FrameId) -> EvalCmdResult;

pub(super) fn eval_builtin(
  name: &str,
  args: &mut [Value],
  context: &mut EvalContext,
  frame: FrameId,
) -> Option<EvalCmdResult> {
  let eval: EvalCmd = match name {
    "break" => cmd_break::eval,
    "continue" => cmd_continue::eval,
    "dict" => cmd_dict::eval,
    "expr" => cmd_expr::eval,
    "foreach" => cmd_foreach::eval,
    "global" => cmd_global::eval,
    "if" => cmd_if::eval,
    "incr" => cmd_incr::eval,
    "info" => cmd_info::eval,
    "lappend" => cmd_lappend::eval,
    "lindex" => cmd_lindex::eval,
    "list" => cmd_list::eval,
    "llength" => cmd_llength::eval,
    "lreverse" => cmd_lreverse::eval,
    "proc" => cmd_proc::eval,
    "puts" => cmd_puts::eval,
    "return" => cmd_return::eval,
    "set" => cmd_set::eval,
    "string" => cmd_string::eval,
    "unknown" => cmd_unknown::eval,
    "uplevel" => cmd_uplevel::eval,
    "upvar" => cmd_upvar::eval,
    "while" => cmd_while::eval,
    _ => return None,
  };

  Some(eval(args, context, frame))
}
