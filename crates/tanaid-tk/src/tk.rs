use crate::cmd;
use crate::tk_context::TkContext;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;

pub struct Tk {
  pub context: Rc<TkContext>,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      context: Rc::new(TkContext::new()),
    }
  }

  pub fn install(&mut self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    cmd::register_commands(ctx, self.context.clone());
    Ok(())
  }
}
