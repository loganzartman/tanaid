#![feature(integer_casts)]

use js_sys::{Date, Function, Promise, Reflect, global};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tanaid::{eval::EvalContext, eval::eval, eval_error::EvalError, parser::parse};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use web_sys;

#[wasm_bindgen]
#[derive(Clone)]
#[expect(dead_code)]
pub struct Interpreter {
  context: Rc<RefCell<EvalContext>>,
  timeout_ids: Rc<RefCell<HashMap<usize, JsValue>>>,
  set_timeout: Function,
  clear_timeout: Function,
  handle_event_loop_status: Function,
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterpreterOptions {
  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(output: string) => void")]
  pub handle_stdout: Function,

  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(callback: () => void, delayMs: number) => unknown")]
  pub set_timeout: Function,

  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(timeoutId: unknown) => void")]
  pub clear_timeout: Function,

  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(pendingTimers: number) => void")]
  pub handle_event_loop_status: Function,
}

fn js_error_message(value: JsValue) -> String {
  value
    .dyn_ref::<js_sys::Error>()
    .and_then(|error| error.message().as_string())
    .or_else(|| value.as_string())
    .unwrap_or_else(|| format!("{value:?}"))
}

fn js_value_to_error(value: JsValue) -> JsError {
  JsError::new(&js_error_message(value))
}

#[wasm_bindgen]
impl Interpreter {
  pub fn create(options: Ts<InterpreterOptions>) -> Result<Interpreter, JsError> {
    console_error_panic_hook::set_once();

    let opts = options
      .to_rust()
      .map_err(|e| JsError::new(format!("failed to parse options: {}", e).as_str()))?;
    let handle_stdout = opts.handle_stdout;

    let stdout = Rc::new(move |value: &str| {
      handle_stdout
        .call(&JsValue::NULL, (&JsValue::from(value),))
        .map_err(|e| EvalError::Generic(js_error_message(e)))?;
      Ok(())
    });

    let set_timeout = opts.set_timeout.clone();
    let sleep_ms = move |ms: u64| {
      let set_timeout = set_timeout.clone();
      async move {
        let done = Promise::new(&mut |resolve, reject| {
          let callback_reject = reject.clone();
          let callback = ScopedClosure::<dyn FnMut()>::own_aborting(move || {
            if let Err(error) = resolve.call1(&JsValue::UNDEFINED, &JsValue::TRUE) {
              let _ = callback_reject.call1(&JsValue::UNDEFINED, &error);
            }
          })
          .into_js_value();

          if let Err(error) = set_timeout.call2(
            &JsValue::UNDEFINED,
            &callback,
            &JsValue::from(ms.saturating_cast::<i32>()),
          ) {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
          }
        });

        done
          .await
          .map(|_| ())
          .map_err(|error| EvalError::Generic(js_error_message(error)))
      }
    };

    let performance = Reflect::get(&global(), &"performance".into())
      .map_err(js_value_to_error)?
      .dyn_into::<web_sys::Performance>()
      .map_err(js_value_to_error)?;

    let clock_monotonic_us = move || {
      let now = performance.now();
      (now * 1000.0).ceil() as u64
    };

    let clock_unixtime_ms = move || {
      let now = Date::now();
      now as i64
    };

    let context = EvalContext::new()
      .with_stdout(stdout)
      .with_sleep_ms(sleep_ms)
      .with_clock_monotonic_us(clock_monotonic_us)
      .with_clock_unixtime_ms(clock_unixtime_ms);

    Ok(Interpreter {
      context: Rc::new(RefCell::new(context)),
      timeout_ids: Rc::new(RefCell::new(HashMap::new())),
      set_timeout: opts.set_timeout,
      clear_timeout: opts.clear_timeout,
      handle_event_loop_status: opts.handle_event_loop_status,
    })
  }

  pub async fn run(&mut self, src: &str) -> Result<JsValue, JsError> {
    let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;

    let mut result = {
      let mut context = self.context.borrow_mut();
      eval(&parsed, &mut *context)
        .await
        .map_err(|e| JsError::new(e.to_string().as_str()))
    }?;
    notify_event_loop_status(self)?;

    match result.repr_str() {
      Ok(result_str) => Ok(JsValue::from_str(result_str)),
      Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
    }
  }

  #[wasm_bindgen(js_name = "runEventLoop")]
  pub async fn run_event_loop(&self) -> Result<(), JsError> {
    while self.context.borrow().count_pending_events() > 0 {
      let Some(delay) = self.context.borrow().next_event_delay()? else {
        break;
      };

      notify_event_loop_status(self)?;

      self
        .context
        .borrow()
        .sleep_ms(delay.as_millis() as u64)
        .await?;

      self.context.borrow_mut().poll_event().await?;
    }
    notify_event_loop_status(self)?;
    Ok(())
  }
}

fn notify_event_loop_status(interpreter: &Interpreter) -> Result<(), JsError> {
  let n_pending = interpreter.context.borrow().count_pending_events();

  interpreter
    .handle_event_loop_status
    .call1(&JsValue::UNDEFINED, &JsValue::from(n_pending))
    .map_err(js_value_to_error)?;

  Ok(())
}
