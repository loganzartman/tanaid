use tanaid::eval_error::EvalError;
use tanaid::value::Value;

use crate::Tk;

pub(crate) fn eval(tk: &Tk, args: &mut [Value]) -> Result<Value, EvalError> {
  let [path, options @ ..] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of args: expected \"pack pathName ?option value ...?\"".to_string(),
    ));
  };

  if !options.is_empty() {
    return Err(EvalError::ArgumentError(
      "unimplemented: \"pack\" ignores no options, so it accepts none".to_string(),
    ));
  }

  let path = path.repr_str()?.to_string();
  let mut state = tk.state.borrow_mut();

  if !state.canvases.contains_key(&path) {
    return Err(EvalError::ArgumentError(format!(
      "bad window path name \"{}\"",
      path
    )));
  }
  if state.mapped.as_ref().is_some_and(|mapped| *mapped != path) {
    return Err(EvalError::ArgumentError(
      "unimplemented: only one widget can be packed".to_string(),
    ));
  }

  state.mapped = Some(path);
  state.dirty = true;

  Ok(Value::none())
}
