use super::tk_context::TkContext;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

mod canvas;
mod pack;

pub(super) type EvalCmdResult = Result<Value, EvalError>;

pub fn register_commands(context: &mut EvalContext, tk: Rc<TkContext>) {
  {
    let tk = tk.clone();
    context.register_command(
      "canvas",
      Rc::new(move |args, ctx, frame| canvas::eval(args, ctx, frame, &tk)),
    );
  }
  {
    let tk = tk.clone();
    context.register_command(
      "pack",
      Rc::new(move |args, ctx, frame| pack::eval(args, ctx, frame, &tk)),
    );
  }
}
