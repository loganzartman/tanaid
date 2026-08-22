use std::cell::Cell;
use std::rc::Rc;
use tanaid::eval::EvalContext;
use tanaid::eval::FrameId;
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub struct TkContext {
  is_packed: Cell<bool>,
}

impl TkContext {
  pub fn new() -> Self {
    Self {
      is_packed: Cell::new(false),
    }
  }

  pub(crate) fn canvas(
    &self,
    args: &mut [Value],
    ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    let path_name = match args {
      [path_name, _rest @ ..] => path_name,
      _ => {
        return Err(EvalError::ArgumentError(
          "canvas: missing path name".to_string(),
        ));
      }
    };

    let path_name_str = path_name.repr_str()?;

    if !path_name_str.starts_with(".") {
      return Err(EvalError::ArgumentError(
        "canvas: path name must start with '.'".to_string(),
      ));
    }

    ctx.register_command(
      path_name_str,
      Rc::new(move |args, ctx, _frame| {
        ctx.write_stdout(format!("canvas: {:?}", args).as_str())?;
        Ok(Value::none())
      }),
    );

    Ok(Value::from(path_name_str))
  }

  pub(crate) fn pack(
    &self,
    _args: &mut [Value],
    _ctx: &mut EvalContext,
    _frame: FrameId,
  ) -> Result<Value, EvalError> {
    self.is_packed.set(true);
    Ok(Value::none())
  }
}
