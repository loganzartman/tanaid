use js_sys::Function;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use tanaid::{
  eval::{EvalContext, eval},
  eval_error::EvalError,
  parser::parse,
};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Interpreter {
  context: EvalContext,
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterpreterOptions {
  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(output: string) => void")]
  pub handle_stdout: Function,
}

#[wasm_bindgen]
impl Interpreter {
  pub fn create(options: Ts<InterpreterOptions>) -> Result<Interpreter, JsError> {
    let opts = options
      .to_rust()
      .map_err(|e| JsError::new(format!("failed to parse options: {}", e).as_str()))?;
    let handle_stdout = opts.handle_stdout;

    let stdout = Rc::new(move |value: &str| {
      handle_stdout
        .call(&JsValue::NULL, (&JsValue::from(value),))
        .map_err(|e| {
          EvalError::Generic(
            e.as_string()
              .unwrap_or("unknown JS error while writing stdout".to_string()),
          )
        })?;
      Ok(())
    });
    let context = EvalContext::new().with_stdout(stdout);

    Ok(Interpreter { context })
  }

  pub fn run(&mut self, src: &str) -> Result<JsValue, JsError> {
    let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;
    let mut result =
      eval(&parsed, &mut self.context).map_err(|e| JsError::new(e.to_string().as_str()))?;
    match result.repr_str() {
      Ok(result_str) => Ok(JsValue::from_str(result_str)),
      Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
    }
  }
}
