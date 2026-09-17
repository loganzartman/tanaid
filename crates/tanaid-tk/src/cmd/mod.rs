pub(crate) mod bind;
pub(crate) mod canvas;
pub(crate) mod canvas_coords;
pub(crate) mod canvas_create;
pub(crate) mod pack;

use super::tk_context::TkContext;
use std::rc::Rc;
use tanaid::eval::EvalContext;

pub fn register_commands(context: &mut EvalContext, tk: Rc<TkContext>) {
  {
    let tk = tk.clone();
    context.register_command("bind", move |args, ctx, frame| {
      bind::eval(args, ctx, frame, &tk)
    });
  }
  {
    let tk = tk.clone();
    context.register_command("canvas", move |args, ctx, frame| {
      canvas::eval(args, ctx, frame, &tk)
    });
  }
  {
    let tk = tk.clone();
    context.register_command("pack", move |args, ctx, frame| {
      pack::eval(args, ctx, frame, &tk)
    });
  }
}
