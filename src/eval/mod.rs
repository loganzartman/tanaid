use crate::eval_error::EvalError;
use crate::parser::ScriptNode;
use crate::value::Value;

mod cmd;
mod cmd_break;
mod cmd_dict;
mod cmd_expr;
mod cmd_foreach;
mod cmd_global;
mod cmd_if;
mod cmd_incr;
mod cmd_info;
mod cmd_lappend;
mod cmd_lindex;
mod cmd_list;
mod cmd_llength;
mod cmd_lreverse;
mod cmd_proc;
mod cmd_puts;
mod cmd_return;
mod cmd_set;
mod cmd_string;
mod cmd_while;
mod context;
mod expr;
mod proc;
mod script;
#[cfg(test)]
mod tests;
mod word;

use context::GLOBAL_FRAME;
pub use context::{Binding, EvalContext, EvalFrame, FrameId};
pub use expr::{eval_expr, eval_expr_binary_op};
pub use proc::{Proc, eval_proc};
pub use script::{eval_command, eval_returnable_script, eval_script};
pub use word::{eval_word, eval_wordpart};

pub fn eval(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  eval_returnable_script(script, context, GLOBAL_FRAME)
}
