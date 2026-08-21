use js_sys::Function;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tanaid::{
  eval::{EvalContext, TimerAction, eval},
  eval_error::EvalError,
  parser::parse,
};
use tanaid_tk::{Renderer, Tk};
use tsify::Ts;
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use web_sys::OffscreenCanvas;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Interpreter {
  context: Rc<RefCell<EvalContext>>,
  timeout_ids: Rc<RefCell<HashMap<usize, JsValue>>>,
  set_timeout: Function,
  clear_timeout: Function,
  handle_event_loop_status: Function,

  tk: Tk,
  /// The canvas to draw on, until it is handed to a renderer.
  canvas: Rc<RefCell<Option<OffscreenCanvas>>>,
  renderer: Rc<RefCell<Option<Renderer>>>,
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

  /// Where Tk draws. A worker can only be handed an `OffscreenCanvas`, so the
  /// page transfers one before running anything that might draw.
  #[serde(default, with = "serde_wasm_bindgen::preserve")]
  #[tsify(type = "OffscreenCanvas | undefined")]
  pub canvas: JsValue,
}

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = console, js_name = error)]
  fn console_error(value: &JsValue);
}

/// A wasm panic aborts with a bare `RuntimeError: unreachable`, and the message
/// std would have printed goes nowhere. Log it — as an `Error`, so the console
/// shows a stack with it — before the process gives up.
fn report_panics_to_console() {
  use std::sync::Once;

  static HOOK: Once = Once::new();
  HOOK.call_once(|| {
    std::panic::set_hook(Box::new(|info| {
      console_error(&js_sys::Error::new(&info.to_string()).into());
    }));
  });
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
    report_panics_to_console();

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
    let mut context = EvalContext::new().with_stdout(stdout);

    let tk = Tk::new();
    tk.register_commands(&mut context);

    let canvas = if opts.canvas.is_undefined() || opts.canvas.is_null() {
      None
    } else {
      Some(
        opts
          .canvas
          .dyn_into::<OffscreenCanvas>()
          .map_err(|_| JsError::new("canvas must be an OffscreenCanvas"))?,
      )
    };

    Ok(Interpreter {
      context: Rc::new(RefCell::new(context)),
      timeout_ids: Rc::new(RefCell::new(HashMap::new())),
      set_timeout: opts.set_timeout,
      clear_timeout: opts.clear_timeout,
      handle_event_loop_status: opts.handle_event_loop_status,

      tk,
      canvas: Rc::new(RefCell::new(canvas)),
      renderer: Rc::new(RefCell::new(None)),
    })
  }

  /// The size of the widget the script mapped, if it opened a window.
  #[wasm_bindgen(js_name = windowSize)]
  pub fn window_size(&self) -> Option<Vec<u32>> {
    self
      .tk
      .mapped_canvas()
      .map(|canvas| vec![canvas.width, canvas.height])
  }

  pub async fn run(&mut self, src: &str) -> Result<JsValue, JsError> {
    let parsed = parse(src).map_err(|e| JsError::new(e.to_string().as_str()))?;

    let mut result = {
      let mut context = self.context.borrow_mut();
      eval(&parsed, &mut *context).map_err(|e| JsError::new(e.to_string().as_str()))
    }?;

    self.open_window().await?;
    self.run_event_loop()?;

    match result.repr_str() {
      Ok(result_str) => Ok(JsValue::from_str(result_str)),
      Err(e) => Err(JsError::new(e.to_string().as_str()).into()),
    }
  }

  /// Attaches a renderer to the canvas, once a script has mapped a widget.
  async fn open_window(&self) -> Result<(), JsError> {
    if !self.tk.has_window() || self.renderer.borrow().is_some() {
      return Ok(());
    }

    let Some(canvas) = self.canvas.borrow_mut().take() else {
      return Err(JsError::new(
        "this script draws, but no canvas was given to the interpreter",
      ));
    };

    if let Some(widget) = self.tk.mapped_canvas() {
      canvas.set_width(widget.width);
      canvas.set_height(widget.height);
    }

    let renderer = tanaid_tk::create_renderer(canvas)
      .await
      .map_err(|e| JsError::new(e.to_string().as_str()))?;
    *self.renderer.borrow_mut() = Some(renderer);

    self.tk.take_dirty();
    self.draw()
  }

  fn draw(&self) -> Result<(), JsError> {
    let mut renderer = self.renderer.borrow_mut();
    let (Some(renderer), Some(canvas)) = (renderer.as_mut(), self.tk.mapped_canvas()) else {
      return Ok(());
    };

    renderer
      .render(&canvas)
      .map_err(|e| JsError::new(e.to_string().as_str()))
  }

  fn run_event_loop(&self) -> Result<(), JsError> {
    let timer_actions = self.context.borrow_mut().take_timer_actions();
    apply_timer_actions(self, timer_actions)?;
    notify_if_event_loop_empty(self)?;

    Ok(())
  }
}

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

          if callback_interpreter.tk.take_dirty()
            && let Err(e) = callback_interpreter.draw()
          {
            wasm_bindgen::throw_val(e.into())
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

fn notify_if_event_loop_empty(interpreter: &Interpreter) -> Result<(), JsError> {
  let n_pending = interpreter.timeout_ids.borrow().len();

  interpreter
    .handle_event_loop_status
    .call1(&JsValue::UNDEFINED, &JsValue::from(n_pending))
    .map_err(js_value_to_error)?;

  Ok(())
}
