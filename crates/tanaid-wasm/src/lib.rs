#![feature(integer_casts)]

use js_sys::{Date, Function, Reflect, global};
use serde::{Deserialize, Serialize};
use std::future::poll_fn;
use std::task::{Poll, Waker};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};
use tanaid::event_loop::EventLoop;
use tanaid::interpreter::{Interpreter, StepResult};
use tanaid::{eval::EvalContext, eval_error::EvalError, parser::parse, value::Value};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use web_sys;

#[wasm_bindgen]
pub struct Tcl {
  interpreter: Interpreter,
  event_loop: Rc<RefCell<EventLoop>>,
  wake_timeout: Option<(JsValue, Closure<dyn FnMut()>)>,
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
  #[serde(default, with = "serde_wasm_bindgen::preserve")]
  #[tsify(optional, type = "(countPending: number) => void")]
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

fn js_value_to_evalerror(value: JsValue) -> EvalError {
  EvalError::Generic(js_error_message(value))
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
      .with_clock_monotonic(clock_monotonic)
      .with_clock_unixtime(clock_unixtime);

    let mut interpreter = Interpreter::new();
    let event_loop = Rc::clone(&context.event_loop);
    interpreter.configure(context);

    Ok(Tcl {
      interpreter,
      event_loop,
      wake_timeout: None,
      set_timeout: opts.set_timeout,
      clear_timeout: opts.clear_timeout,
    })
  }

  pub async fn run(
    &mut self,
    src: &str,
    options: Option<Ts<RunOptions>>,
  ) -> Result<JsValue, JsError> {
    if self.interpreter.is_busy() {
      return Err(JsError::new("interpreter is already running a script"));
    }

    let options = options
      .map(|o| o.to_rust())
      .transpose()
      .map_err(|e| JsError::new(format!("failed to parse options: {}", e).as_str()))?;
    let handle_event_loop_status = options.map(|o| o.handle_event_loop_status);

    let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;

    self.interpreter.start(&parsed)?;

    let result: Result<Value, EvalError> = poll_fn(|cx| {
      let step = self.interpreter.step(cx.waker())?;

      if let Some(handler) = &handle_event_loop_status {
        let count_pending = self.event_loop.borrow().count_pending();
        handler
          .call1(
            &JsValue::UNDEFINED,
            &JsValue::from(count_pending.saturating_cast::<i32>()),
          )
          .map_err(js_value_to_evalerror)?;
      }

      match step {
        // need to do more work; yield and re-run immediately
        StepResult::Again => {
          self.set_wake_timeout(cx.waker(), Duration::ZERO)?;
          Poll::Pending
        }
        // done; resolve future
        StepResult::Done(value) => Poll::Ready(Ok(value)),
        // scheduled timer; re-run after duration if nothing wakes us earlier
        StepResult::WaitDuration(duration) => {
          self.set_wake_timeout(cx.waker(), duration)?;
          Poll::Pending
        }
        // nothing scheduled; wait for external wake
        StepResult::Wait => Poll::Pending,
      }
    })
    .await;

    match result?.repr_str() {
      Ok(s) => Ok(JsValue::from_str(s)),
      Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
    }
  }

  fn set_wake_timeout(&mut self, waker: &Waker, duration: Duration) -> Result<(), EvalError> {
    // avoid accumulating timeouts
    if let Some((id, _closure)) = self.wake_timeout.take() {
      // clearTimeout failure shouldn't occur; if it does, continue.
      let _ = self.clear_timeout.call1(&JsValue::UNDEFINED, &id);
    }

    let waker = waker.clone();
    let closure = Closure::once(move || waker.wake());
    let id = self
      .set_timeout
      .call2(
        &JsValue::UNDEFINED,
        closure.as_ref(),
        &JsValue::from(duration.as_millis().saturating_cast::<i32>()),
      )
      .map_err(js_value_to_evalerror)?;
    self.wake_timeout = Some((id, closure));

    Ok(())
  }
}

impl Drop for Tcl {
  fn drop(&mut self) {
    if let Some((id, _closure)) = self.wake_timeout.take() {
      let _ = self.clear_timeout.call1(&JsValue::UNDEFINED, &id);
    }
  }
}
