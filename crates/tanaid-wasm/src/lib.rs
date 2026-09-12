use js_sys::{Function, Promise};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tanaid::{eval::EvalContext, eval::eval, eval_error::EvalError, parser::parse};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

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
        let done = Promise::new(&mut |res, _rej| {
          set_timeout
            .call2(
              &JsValue::UNDEFINED,
              &ScopedClosure::<dyn FnMut()>::own_aborting(move || {
                res
                  .call1(&JsValue::UNDEFINED, &JsValue::from(true))
                  .expect("failed to sleep");
              })
              .into_js_value(),
              &JsValue::from(ms as i32),
            )
            .expect("failed to sleep");
        });
        done.await.expect("failed to sleep");
      }
    };

    let context = EvalContext::new()
      .with_stdout(stdout)
      .with_sleep_ms(sleep_ms);

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

    self.run_event_loop().await?;

    match result.repr_str() {
      Ok(result_str) => Ok(JsValue::from_str(result_str)),
      Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
    }
  }

  async fn run_event_loop(&self) -> Result<(), JsError> {
    while self.context.borrow().count_pending_events() > 0 {
      let Some(delay) = self.context.borrow().next_event_delay() else {
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

/*
fn apply_timer_actions(
  interpreter: &Interpreter,
  timer_actions: Vec<TimerAction>,
) -> Result<(), JsError> {
  for action in timer_actions {
    match action {
      TimerAction::Start { timer_id, delay_ms } => {
        let Ok(delay_ms_i32) = i32::try_from(delay_ms) else {
          wasm_bindgen::throw_val(JsError::new("invalid delay").into())
        };

        let callback_interpreter = interpreter.clone();

        let callback = ScopedClosure::<dyn FnMut()>::own_aborting(move || {
          let fire_result = callback_interpreter
            .context
            .borrow_mut()
            .fire_timer(timer_id);

          callback_interpreter
            .timeout_ids
            .borrow_mut()
            .remove(&timer_id);

          let timer_actions = callback_interpreter
            .context
            .borrow_mut()
            .take_timer_actions();

          match apply_timer_actions(&callback_interpreter, timer_actions) {
            Err(e) => wasm_bindgen::throw_val(e.into()),
            Ok(()) => {}
          }

          if let Err(e) = fire_result {
            wasm_bindgen::throw_val(JsError::new(e.to_string().as_str()).into())
          }

          if let Err(e) = notify_if_event_loop_empty(&callback_interpreter) {
            wasm_bindgen::throw_val(e.into())
          }
        })
        .into_js_value();

        let timeout_id: JsValue = interpreter
          .set_timeout
          .call2(
            &JsValue::UNDEFINED,
            callback.as_ref(),
            &JsValue::from(delay_ms_i32),
          )
          .map_err(js_value_to_error)?;

        interpreter
          .timeout_ids
          .borrow_mut()
          .insert(timer_id, timeout_id);
      }
      TimerAction::Cancel { timer_id } => {
        let timeout_ids = interpreter.timeout_ids.clone();

        {
          let timeout_ids = timeout_ids.borrow();
          let timeout_id = timeout_ids.get(&timer_id);

          if let Some(timeout_id) = timeout_id {
            interpreter
              .clear_timeout
              .call1(&JsValue::UNDEFINED, timeout_id)
              .map_err(js_value_to_error)?;
          }
        }

        timeout_ids.borrow_mut().remove(&timer_id);
      }
    }
  }

  Ok(())
}
*/

fn notify_event_loop_status(interpreter: &Interpreter) -> Result<(), JsError> {
  let n_pending = interpreter.context.borrow().count_pending_events();

  interpreter
    .handle_event_loop_status
    .call1(&JsValue::UNDEFINED, &JsValue::from(n_pending))
    .map_err(js_value_to_error)?;

  Ok(())
}
