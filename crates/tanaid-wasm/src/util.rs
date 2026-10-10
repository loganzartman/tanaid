use tanaid::eval_error::EvalError;
use wasm_bindgen::{JsCast, JsError, JsValue};

pub(crate) fn js_error_message(value: JsValue) -> String {
  value
    .dyn_ref::<js_sys::Error>()
    .and_then(|error| error.message().as_string())
    .or_else(|| value.as_string())
    .unwrap_or_else(|| format!("{value:?}"))
}

pub(crate) fn js_value_to_error(value: JsValue) -> JsError {
  JsError::new(&js_error_message(value))
}

pub(crate) fn js_value_to_evalerror(value: JsValue) -> EvalError {
  EvalError::Generic(js_error_message(value))
}
