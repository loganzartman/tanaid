use super::{EvalContext, FrameId, cmd::EvalCmdResult};
use crate::eval::eval_script;
use crate::eval_error::EvalError;
use crate::value::{List, Value};

const WRONG_ARGS_MSG: &str =
  "wrong number of arguments; expects: foreach varlist1 list1 ?varlist2 list2 ...? body";

pub(super) fn eval(args: &mut [Value], context: &mut EvalContext, frame: FrameId) -> EvalCmdResult {
  let [varlists_lists_args @ .., body] = args else {
    return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
  };

  // eval args
  let mut varlists_lists: Vec<(Vec<String>, List)> = vec![];
  let mut vl_it = varlists_lists_args.iter_mut();
  loop {
    let Some(varlist_val) = vl_it.next() else {
      break;
    };
    let Some(list) = vl_it.next() else {
      return Err(EvalError::ArgumentError(WRONG_ARGS_MSG.to_string()));
    };

    let mut varlist: Vec<String> = vec![];
    let mut varlist_elems = varlist_val.repr_list()?.as_ref().clone();
    for val in varlist_elems.iter_mut() {
      varlist.push(val.repr_str()?.to_string());
    }
    if varlist.is_empty() {
      return Err(EvalError::ArgumentError(
        "foreach varlist can't be empty".to_string(),
      ));
    }

    varlists_lists.push((varlist, list.repr_list()?.as_ref().clone()));
  }

  let body_str = body.repr_str()?;
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
