use crate::tk_context::TkContext;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::{EvalContext, FrameId};
use tanaid::eval_error::EvalError;
use tanaid::value::Value;

pub(super) async fn eval(
  args: &mut [Value],
  ctx: &mut EvalContext,
  _frame: FrameId,
  tk: &TkContext,
) -> EvalCmdResult {
  match args {
    [subcommand, rest @ ..] => {
      let subcommand_str = subcommand.repr_str()?;
      match subcommand_str {
        "window" => eval_window(rest, ctx, tk).await,
        _ => Err(EvalError::ArgumentError(format!(
          "tkwait: unsupported subcommand: {}",
          subcommand_str
        ))),
      }
    }
    _ => Err(EvalError::ArgumentError(
      "tkwait: expected subcommand".to_string(),
    )),
  }
}

async fn eval_window(args: &mut [Value], ctx: &mut EvalContext, tk: &TkContext) -> EvalCmdResult {
  let window = match args {
    [window] => window.repr_str()?,
    _ => {
      return Err(EvalError::ArgumentError(
        "tkwait window: expected window name".to_string(),
      ));
    }
  };

  if window != "." {
    return Err(EvalError::ArgumentError(
      "tkwait window: only window . is supported".to_string(),
    ));
  }

  while tk.has_window() {
    ctx.poll_event().await?;
  }

  Ok(Value::none())
}
