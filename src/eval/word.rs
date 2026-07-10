use super::{EvalContext, FrameId, eval_script};
use crate::eval_error::EvalError;
use crate::parser::{WordNode, WordPart};
use crate::value::Value;

pub fn eval_word(
  word: &WordNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  // optimization for single-part words
  if let [part] = word.parts.as_slice() {
    return eval_wordpart(part, context, frame);
  }

  let mut joined = String::new();
  for part in &word.parts {
    let mut value = eval_wordpart(part, context, frame)?;
    joined.push_str(value.repr_str()?);
  }
  Ok(Value::new(joined))
}

pub fn eval_wordpart(
  part: &WordPart,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  match part {
    WordPart::BareLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedSub(v) => context
      .get_variable(frame, &v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::CommandSub(c) => {
      let parsed = context
        .parse_script_caching(c)
        .map_err(|e| EvalError::CommandParseError(e.to_string()))?;
      let (script, _) = parsed.as_ref();
      eval_script(script, context, frame)
    }
    WordPart::Quoted(parts) => {
      let mut result: String = "".to_string();
      for part in parts {
        let mut string = eval_wordpart(part, context, frame)?;
        result.push_str(string.repr_str()?);
      }
      Ok(Value::new(result))
    }
    WordPart::VarSub(v) => context
      .get_variable(frame, &v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::VarIndex(_, _) => Err(EvalError::NotImplemented),
    WordPart::BracedIndex(_, _) => Err(EvalError::NotImplemented),
  }
}
