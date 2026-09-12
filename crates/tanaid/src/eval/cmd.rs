use super::{
  EvalContext, cmd_after, cmd_break, cmd_continue, cmd_dict, cmd_expr, cmd_foreach, cmd_global,
  cmd_if, cmd_incr, cmd_info, cmd_lappend, cmd_lassign, cmd_lindex, cmd_list, cmd_llength,
  cmd_lreverse, cmd_lset, cmd_package, cmd_proc, cmd_puts, cmd_return, cmd_set, cmd_string,
  cmd_unknown, cmd_uplevel, cmd_upvar, cmd_vwait, cmd_while,
};
use crate::eval::cmd_update;
use crate::eval_error::EvalError;
use crate::value::Value;

pub type EvalCmdResult = Result<Value, EvalError>;

pub fn register_builtin_commands(context: &mut EvalContext) {
  context.register_async_command("after", cmd_after::eval);
  context.register_command("break", cmd_break::eval);
  context.register_command("continue", cmd_continue::eval);
  context.register_command("dict", cmd_dict::eval);
  context.register_async_command("expr", cmd_expr::eval);
  context.register_async_command("foreach", cmd_foreach::eval);
  context.register_command("global", cmd_global::eval);
  context.register_async_command("if", cmd_if::eval);
  context.register_command("incr", cmd_incr::eval);
  context.register_command("info", cmd_info::eval);
  context.register_command("lappend", cmd_lappend::eval);
  context.register_command("lassign", cmd_lassign::eval);
  context.register_command("lindex", cmd_lindex::eval);
  context.register_command("list", cmd_list::eval);
  context.register_command("llength", cmd_llength::eval);
  context.register_command("lreverse", cmd_lreverse::eval);
  context.register_command("lset", cmd_lset::eval);
  context.register_command("package", cmd_package::eval);
  context.register_command("proc", cmd_proc::eval);
  context.register_command("puts", cmd_puts::eval);
  context.register_command("return", cmd_return::eval);
  context.register_command("set", cmd_set::eval);
  context.register_command("string", cmd_string::eval);
  context.register_command("unknown", cmd_unknown::eval);
  context.register_async_command("update", cmd_update::eval);
  context.register_async_command("uplevel", cmd_uplevel::eval);
  context.register_command("upvar", cmd_upvar::eval);
  context.register_async_command("vwait", cmd_vwait::eval);
  context.register_async_command("while", cmd_while::eval);
}
