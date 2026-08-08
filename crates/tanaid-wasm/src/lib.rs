use tanaid::{
  eval::{EvalContext, eval},
  parser::parse,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_tcl(src: &str) -> Result<JsValue, JsError> {
  let mut context = EvalContext::new();
  let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;
  let mut result = eval(&parsed, &mut context).map_err(|e| JsError::new(e.to_string().as_str()))?;
  match result.repr_str() {
    Ok(result_str) => Ok(JsValue::from_str(result_str)),
    Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
  }
}
