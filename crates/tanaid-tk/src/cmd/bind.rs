use crate::events::parse_sequence;
use crate::tk_context::TkContext;
use tanaid::eval::EvalCmdResult;
use tanaid::eval::{EvalContext, FrameId};
use tanaid::eval_error::EvalError;
use tanaid::parser;
use tanaid::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  _ctx: &mut EvalContext,
  _frame: FrameId,
  tk: &TkContext,
) -> EvalCmdResult {
  match args {
    [tag, sequence, script] => {
      let tag_str = tag.repr_str()?;
      let sequence_str = sequence.repr_str()?;
      let script_str = script.repr_str()?;
      let (script_str, append) = if let Some(stripped) = script_str.strip_prefix("+") {
        (stripped, true)
      } else {
        (script_str, false)
      };

      let sequence = parse_sequence(sequence_str).map_err(|e| EvalError::Generic(e.to_string()))?;
      let script =
        parser::parse(script_str).map_err(|e| EvalError::ScriptParseError(e.to_string()))?;

      tk.event_bindings
        .borrow_mut()
        .bind(tag_str.to_string(), sequence, &script, append);
    }
    _ => {
      return Err(EvalError::ArgumentError("bind invalid args".to_string()));
    }
  }
  Ok(Value::none())
}
