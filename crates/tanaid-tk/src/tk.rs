use crate::cmd;
use crate::tk_context::TkContext;
use tanaid::eval::EvalContext;
use tanaid::eval_error::EvalError;

pub struct Tk {
  pub context: TkContext,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      context: TkContext::new(),
    }
  }

  pub fn install(&mut self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    cmd::register_commands(ctx, &self.context);
    Ok(())
  }
}
