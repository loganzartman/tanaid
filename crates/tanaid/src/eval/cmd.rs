use super::{
  EvalContext, cmd_break, cmd_continue, cmd_dict, cmd_expr, cmd_foreach, cmd_global, cmd_if,
  cmd_incr, cmd_info, cmd_lappend, cmd_lindex, cmd_list, cmd_llength, cmd_lreverse, cmd_proc,
  cmd_puts, cmd_return, cmd_set, cmd_string, cmd_uplevel, cmd_upvar, cmd_while,
};
use crate::eval::cmd_unknown;
use crate::eval_error::EvalError;
use crate::value::Value;
use std::rc::Rc;

pub(super) type EvalCmdResult = Result<Value, EvalError>;

pub fn register_builtin_commands(context: &mut EvalContext) {
  context.register_command("break", Rc::new(cmd_break::eval));
  context.register_command("continue", Rc::new(cmd_continue::eval));
  context.register_command("dict", Rc::new(cmd_dict::eval));
  context.register_command("expr", Rc::new(cmd_expr::eval));
  context.register_command("foreach", Rc::new(cmd_foreach::eval));
  context.register_command("global", Rc::new(cmd_global::eval));
  context.register_command("if", Rc::new(cmd_if::eval));
  context.register_command("incr", Rc::new(cmd_incr::eval));
  context.register_command("info", Rc::new(cmd_info::eval));
  context.register_command("lappend", Rc::new(cmd_lappend::eval));
  context.register_command("lindex", Rc::new(cmd_lindex::eval));
  context.register_command("list", Rc::new(cmd_list::eval));
  context.register_command("llength", Rc::new(cmd_llength::eval));
  context.register_command("lreverse", Rc::new(cmd_lreverse::eval));
  context.register_command("proc", Rc::new(cmd_proc::eval));
  context.register_command("puts", Rc::new(cmd_puts::eval));
  context.register_command("return", Rc::new(cmd_return::eval));
  context.register_command("set", Rc::new(cmd_set::eval));
  context.register_command("string", Rc::new(cmd_string::eval));
  context.register_command("unknown", Rc::new(cmd_unknown::eval));
  context.register_command("uplevel", Rc::new(cmd_uplevel::eval));
  context.register_command("upvar", Rc::new(cmd_upvar::eval));
  context.register_command("while", Rc::new(cmd_while::eval));
}
