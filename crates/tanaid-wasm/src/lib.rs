#![feature(integer_casts)]

use js_sys::{Date, Function, Promise, Reflect, global};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tanaid::interpreter::{Interpreter, StepResult};
use tanaid::{eval::EvalContext, eval_error::EvalError, parser::parse};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use web_sys;

#[wasm_bindgen]
#[expect(dead_code)]
pub struct Tcl {
  interpreter: Interpreter,
  timeout_ids: Rc<RefCell<HashMap<usize, JsValue>>>,
  set_timeout: Function,
  clear_timeout: Function,
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TclOptions {
  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(output: string) => void")]
  pub handle_stdout: Function,

  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(callback: () => void, delayMs: number) => unknown")]
  pub set_timeout: Function,

  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(timeoutId: unknown) => void")]
  pub clear_timeout: Function,
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunOptions {
  #[serde(with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "(countPending: number) => void")]
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
impl Tcl {
  pub fn create(options: Ts<TclOptions>) -> Result<Tcl, JsError> {
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

    let clock_monotonic = move || {
      let now = performance.now();
      Duration::from_micros((now * 1000.0).ceil() as u64)
    };

    let clock_unixtime = move || {
      let now = Date::now();
      Duration::from_millis(now as u64)
    };

    let context = EvalContext::new()
      .with_stdout(stdout)
      .with_sleep_ms(sleep_ms)
      .with_clock_monotonic(clock_monotonic)
      .with_clock_unixtime(clock_unixtime);

    let mut interpreter = Interpreter::new();
    interpreter.configure(context);

    Ok(Tcl {
      interpreter,
      timeout_ids: Rc::new(RefCell::new(HashMap::new())),
      set_timeout: opts.set_timeout,
      clear_timeout: opts.clear_timeout,
    })
  }

  pub async fn run(&mut self, src: &str, _options: Ts<RunOptions>) -> Result<JsValue, JsError> {
    if self.interpreter.is_busy() {
      return Err(JsError::new("interpreter is already running a script"));
    }

    let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;

    self.interpreter.start(&parsed)?;

    let set_timeout = self.set_timeout.clone();
    let sleep = move |duration: Duration| {
      Promise::new(&mut |resolve, reject| {
        if let Err(error) = set_timeout.call2(
          &JsValue::UNDEFINED,
          &resolve,
          &JsValue::from(duration.as_millis().saturating_cast::<i32>()),
        ) {
          let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
      })
    };

    let mut result = None;
    loop {
      let step = self.interpreter.step()?;
      match step {
        StepResult::Again => continue,
        StepResult::Done(value) => {
          result = result.or(Some(value));
          break;
        }
        StepResult::Wait => {
          sleep(Duration::ZERO).await.map_err(js_value_to_error)?;
        }
        StepResult::WaitDuration(duration) => {
          sleep(duration).await.map_err(js_value_to_error)?;
        }
      }

      if !self.interpreter.is_busy() {
        break;
      }
    }

    match result {
      Some(mut v) => match v.repr_str() {
        Ok(s) => Ok(JsValue::from_str(s)),
        Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
      },
      None => Ok(JsValue::UNDEFINED),
    }
  }
}
