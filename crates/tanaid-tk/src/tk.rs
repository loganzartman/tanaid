use crate::tk_context::TkContext;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;

pub struct Tk {
  context: Rc<TkContext>,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      context: Rc::new(TkContext::new()),
    }
  }

  pub fn install(&self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    {
      let context = self.context.clone();
      ctx.register_command(
        "canvas",
        Rc::new(move |args, ctx, frame| context.canvas(args, ctx, frame)),
      );
    }
    {
      let context = self.context.clone();
      ctx.register_command(
        "pack",
        Rc::new(move |args, ctx, frame| context.pack(args, ctx, frame)),
      );
    }

    Ok(())
  }
}
