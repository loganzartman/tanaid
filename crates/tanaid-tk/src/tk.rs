use std::cell::Cell;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub struct Tk {
  context: Rc<TkContext>,
}

pub struct TkContext {
  is_packed: Cell<bool>,
}

impl Tk {
  pub fn new() -> Self {
    Self {
      context: Rc::new(TkContext {
        is_packed: Cell::new(false),
      }),
    }
  }

  pub fn install(&self, ctx: &mut EvalContext) -> Result<(), EvalError> {
    let context = self.context.clone();
    ctx.register_command(
      "pack",
      Rc::new(move |args, ctx, frame| context.pack(args, ctx, frame)),
    );

    Ok(())
  }
}

impl TkContext {
  fn pack(
    &self,
    _args: &mut [Value],
    _ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    self.is_packed.set(true);
    Ok(Value::none())
  }
}
