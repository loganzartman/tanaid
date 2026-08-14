use super::{cmd::EvalCmdResult, proc::ProcParam, EvalContext, FrameId, Proc};
use crate::eval_error::EvalError;
use crate::parser::{self};
use crate::value::Value;

pub(super) fn eval(
  args: &mut [Value],
  context: &mut EvalContext,
  _frame: FrameId,
) -> EvalCmdResult {
  let [name_val, params_val, body_val] = args else {
    return Err(EvalError::ArgumentError(
      "wrong number of arguments; expects: proc name params body".to_string(),
    ));
  };

  let name = name_val.repr_str()?;

  // args list items are not eval'ed; parse list and convert to literal strings
  let (params_raw, "") = parser::parse_list(params_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc params must be a list: {}", e)))?
  else {
    return Err(EvalError::ArgumentError(
      "proc params must be a list: trailing input".to_string(),
    ));
  };

  let params = params_raw
    .iter()
    .map(|param_raw| {
      let (list, "") = parser::parse_list(param_raw).map_err(|e| {
        EvalError::ArgumentError(format!("invalid parameter \"{}\": {}", param_raw, e))
      })?
      else {
        return Err(EvalError::ArgumentError(format!(
          "invalid parameter \"{}\"",
          param_raw
        )));
      };

      match list.as_slice() {
        [param] => Ok(ProcParam {
          name: param.clone(),
          default: None,
        }),
        [param, default] => Ok(ProcParam {
          name: param.clone(),
          default: Some(default.clone()),
        }),
        _ => Err(EvalError::ArgumentError(format!(
          "too many fields in argument specifier \"{}\"",
          param_raw
        ))),
      }
    })
    .collect::<Result<Vec<_>, _>>()?;

  let parsed = context
    .parse_script_caching(body_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc body must be a script: {}", e)))?;
  let (body, rest) = parsed.as_ref();

  if !rest.is_empty() {
    return Err(EvalError::ArgumentError(
      "proc body must be a script: trailing input".to_string(),
    ));
  };

  let body = body.clone();
  context.set_proc(name, Proc { params, body });
  Ok(Value::none())
}
