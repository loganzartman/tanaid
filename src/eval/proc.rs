use super::{EvalContext, FrameId, eval_returnable_script, eval_word};
use crate::eval_error::EvalError;
use crate::parser::{ScriptNode, WordNode};
use crate::value::Value;

#[derive(PartialEq, Clone, Debug)]
pub struct Proc {
  pub(crate) params: Vec<String>,
  pub(crate) body: ScriptNode,
}

pub fn eval_proc(
  name: &str,
  proc: &Proc,
  args: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  context.run_with_frame(frame, |context, proc_frame| {
    // bind arguments
    let mut args_it = args.iter();
    for (i, param) in proc.params.iter().enumerate() {
      // handle rest args
      if i == proc.params.len() - 1 {
        if param == "args" {
          let args_concat = args_it
            .by_ref()
            .map(|word| {
              eval_word(word, context, frame)?
                .repr_str()
                .map(|str| str.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(" ");
          context.set_variable(proc_frame, "args", Value::new(args_concat));
          break;
        }
      }

      let value = args_it
        .next()
        .map(|w| eval_word(w, context, frame))
        .ok_or_else(|| EvalError::ArgumentError(format!("not enough args for {}", name)))??;
      context.set_variable(proc_frame, param, value);
    }

    if args_it.next().is_some() {
      return Err(EvalError::ArgumentError(format!(
        "too many args for {}",
        name
      )));
    }

    eval_returnable_script(&proc.body, context, proc_frame)
  })
}
