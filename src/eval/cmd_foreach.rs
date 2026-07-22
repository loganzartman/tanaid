use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::{eval_script, eval_word};
use crate::eval_error::EvalError;
use crate::parser::WordNode;
use crate::value::{List, Value};

const WRONG_ARGS_MSG: &str =
  "wrong number of arguments; expects: foreach varlist1 list1 ?varlist2 list2 ...? body";

pub(super) fn eval(words: &[WordNode], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [varlists_lists_words @ .., body] = words else {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  };

  // eval args
  let mut varlists_lists: Vec<(Vec<String>, List)> = vec![];
  let mut vl_it = varlists_lists_words.iter();
  loop {
    let Some(varlist_word) = vl_it.next() else {
      break;
    };
    let Some(list_word) = vl_it.next() else {
      return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
    };

    let mut varlist_vals = eval_word(varlist_word, context, frame)?
      .repr_list()?
      .as_ref()
      .clone();
    let mut varlist: Vec<String> = vec![];
    for val in varlist_vals.iter_mut() {
      varlist.push(val.repr_str()?.to_string());
    }
    if varlist.is_empty() {
      return Err(EvalError::ArgumentError(
        "foreach varlist can't be empty".to_string(),
      ));
    }

    let list = eval_word(list_word, context, frame)?.repr_list()?;

    varlists_lists.push((varlist, list.as_ref().clone()));
  }

  let mut body_val = eval_word(body, context, frame)?;
  let body_str = body_val.repr_str()?;
  let (body_script, _) = context
    .parse_script_caching(body_str)
    .map_err(|e| EvalError::ScriptParseError(e.to_string()))?
    .as_ref()
    .clone();

  for i in 0.. {
    // stop if all lists are exhausted
    if varlists_lists
      .iter()
      .all(|(varlist, list)| list.len() <= i * varlist.len())
    {
      break;
    }

    // bind variables
    for (varlist, list) in &varlists_lists {
      for (j, var) in varlist.iter().enumerate() {
        let value = match list.get(i * varlist.len() + j) {
          Some(value) => value.clone(),
          None => Value::none(),
        };
        context.set_variable(frame, var.as_str(), value);
      }
    }

    // execute body
    match eval_script(&body_script, context, frame) {
      Ok(_) => Ok(()),
      Err(EvalError::BreakError) => break,
      Err(EvalError::ContinueError) => Ok(()),
      Err(err) => Err(err),
    }?
  }
  Ok(Value::none())
}
