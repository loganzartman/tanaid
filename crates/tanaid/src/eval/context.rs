use super::Proc;
use super::event_loop::EventLoop;
use crate::eval::{EvalCmdResult, eval_returnable_script};
use crate::eval_error::EvalError;
use crate::parser::{self, ParseError, ScriptNode};
use crate::parser_expr::{self, ExprNode};
use crate::value::Value;
use lru::LruCache;
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

pub type FrameId = usize;
pub(crate) const GLOBAL_FRAME: FrameId = 0;

pub type OutputSink = Rc<dyn Fn(&str) -> Result<(), EvalError>>;
pub type CommandHandler = dyn for<'a> Fn(
  &'a mut [Value],
  &'a mut EvalContext,
  FrameId,
) -> Pin<Box<dyn Future<Output = EvalCmdResult> + 'a>>;

#[derive(Clone)]
pub struct EvalContext {
  procs: HashMap<String, Rc<Proc>>,
  commands: HashMap<String, Rc<CommandHandler>>,
  frame_id: usize,
  frames: HashMap<FrameId, EvalFrame>,
  event_loop: Rc<RefCell<EventLoop>>,
  sleep_ms: Option<Rc<dyn Fn(u64) -> Pin<Box<dyn Future<Output = ()>>> + 'static>>,

  parse_cache_script: LruCache<String, Rc<(ScriptNode, String)>>,
  parse_cache_expr: LruCache<String, Rc<(ExprNode, String)>>,

  pub(crate) stdout: OutputSink,
}

#[derive(Clone, Debug)]
pub struct EvalFrame {
  caller: Option<FrameId>,
  variables: HashMap<String, Binding>,
}

#[derive(Clone, Debug)]
pub enum Binding {
  Val(Value),
  Ref(FrameId, String),
}

impl std::fmt::Debug for EvalContext {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EvalContext")
      .field("procs", &self.procs)
      .field("frames", &self.frames)
      .field("parse_cache_script", &self.parse_cache_script)
      .field("parse_cache_expr", &self.parse_cache_expr)
      .field("stdout", &"<output sink>")
      .finish()
  }
}

impl EvalContext {
  pub fn new() -> EvalContext {
    let mut context = EvalContext {
      procs: HashMap::new(),
      commands: HashMap::new(),
      frame_id: GLOBAL_FRAME,
      frames: HashMap::from([(GLOBAL_FRAME, EvalFrame::new())]),
      event_loop: Rc::new(RefCell::new(EventLoop::new())),
      sleep_ms: None,

      parse_cache_script: LruCache::new(NonZeroUsize::new(1024).unwrap()),
      parse_cache_expr: LruCache::new(NonZeroUsize::new(1024).unwrap()),
      stdout: Rc::new(|output: &str| {
        print!("{}", output);
        Ok(())
      }),
    };
    super::cmd::register_builtin_commands(&mut context);
    context
  }

  pub fn with_sleep_ms<F, Fut>(mut self, f: F) -> Self
  where
    F: Fn(u64) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
  {
    self.sleep_ms = Some(Rc::new(move |ms: u64| Box::pin(f(ms))));
    self
  }

  pub fn with_event_loop(self, event_loop: EventLoop) -> Self {
    self.event_loop.replace(event_loop);
    self
  }

  #[cfg(not(target_family = "wasm"))]
  pub fn with_std_time(self) -> Self {
    self
      .with_event_loop(EventLoop::new().with_std_time())
      .with_sleep_ms(async |ms| {
        std::thread::sleep(std::time::Duration::from_millis(ms));
      })
  }

  pub fn with_stdout(mut self, stdout: OutputSink) -> Self {
    self.stdout = stdout;
    self
  }

  pub fn write_stdout(&self, output: &str) -> Result<(), EvalError> {
    (self.stdout)(output)
  }

  pub fn frame(&self, id: FrameId) -> &EvalFrame {
    self.frames.get(&id).unwrap()
  }

  pub fn frame_mut(&mut self, id: FrameId) -> &mut EvalFrame {
    self.frames.get_mut(&id).unwrap()
  }

  pub fn frameid_relative(&self, id: FrameId, up: usize) -> Option<FrameId> {
    let mut id = id;
    let mut up = up;

    while up > 0 {
      let Some(frame) = self.frames.get(&id) else {
        return None;
      };
      let Some(next) = frame.caller else {
        return None;
      };
      id = next;
      up -= 1;
    }

    Some(id)
  }

  pub fn frameid_absolute(&self, id: FrameId, depth: usize) -> Option<FrameId> {
    let mut id = id;
    let mut frames: Vec<FrameId> = vec![];

    loop {
      frames.push(id);
      if id == GLOBAL_FRAME {
        break;
      }

      let Some(frame) = self.frames.get(&id) else {
        return None;
      };
      let Some(next) = frame.caller else {
        return None;
      };
      id = next;
    }

    if let Some(frameid) = frames.iter().rev().nth(depth) {
      Some(*frameid)
    } else {
      None
    }
  }

  pub async fn run_with_frame<R>(
    &mut self,
    calling_frame: FrameId,
    f: impl AsyncFnOnce(&mut EvalContext, FrameId) -> R,
  ) -> R {
    let frame_id = self.frame_id + 1;
    self.frame_id = frame_id;

    self
      .frames
      .insert(frame_id, EvalFrame::new_from(calling_frame));

    let result = f(self, frame_id).await;

    self.frames.remove(&frame_id);

    result
  }

  pub fn get_command(&self, name: &str) -> Option<Rc<CommandHandler>> {
    self.commands.get(name).cloned()
  }

  pub fn register_command(
    &mut self,
    name: &str,
    handler: impl for<'a> Fn(&'a mut [Value], &'a mut EvalContext, FrameId) -> EvalCmdResult + 'static,
  ) {
    let handler = Rc::new(handler);
    let wrapped: Rc<CommandHandler> = Rc::new(move |values, context, frame_id| {
      let handler = handler.clone();
      Box::pin(async move { handler(values, context, frame_id) })
    });
    self.commands.insert(name.to_string(), wrapped);
  }

  pub fn register_async_command(
    &mut self,
    name: &str,
    handler: impl for<'a> AsyncFn(&'a mut [Value], &'a mut EvalContext, FrameId) -> EvalCmdResult
    + 'static,
  ) {
    let handler = Rc::new(handler);
    let wrapped: Rc<CommandHandler> = Rc::new(move |values, context, frame_id| {
      let handler = handler.clone();
      Box::pin(async move { handler(values, context, frame_id).await })
    });
    self.commands.insert(name.to_string(), wrapped);
  }

  pub fn unregister_command(&mut self, name: &str) {
    self.commands.remove(name);
  }

  pub fn get_proc(&self, name: &str) -> Option<Rc<Proc>> {
    self.procs.get(name).cloned()
  }

  pub fn set_proc(&mut self, name: &str, proc: Proc) {
    self.procs.insert(name.to_string(), Rc::new(proc));
  }

  pub fn start_timer(
    &mut self,
    timer_script: ScriptNode,
    delay_ms: u64,
  ) -> Result<usize, EvalError> {
    return Ok(
      self
        .event_loop
        .borrow_mut()
        .start_timer(timer_script, delay_ms),
    );
  }

  pub fn cancel_timer(&mut self, timer_id: usize) -> Result<bool, EvalError> {
    self.event_loop.borrow_mut().cancel_timer(timer_id);
    Ok(true)
  }

  pub async fn sleep_ms(&self, ms: u64) -> Result<(), EvalError> {
    let sleep_ms = self.sleep_ms.as_ref().ok_or_else(|| {
      EvalError::Generic(
        "environment does not support timers (missing callback_sleep_ms)".to_string(),
      )
    })?;
    sleep_ms(ms).await;
    Ok(())
  }

  pub fn count_pending_events(&self) -> usize {
    self.event_loop.borrow().count_pending()
  }

  pub fn next_event_delay(&self) -> Option<Duration> {
    self.event_loop.borrow_mut().next_delay()
  }

  pub fn clock_monotonic(&self) -> Duration {
    self.event_loop.borrow().clock_monotonic()
  }

  pub async fn poll_event(&mut self) -> Result<bool, EvalError> {
    let Some((_, script)) = self.event_loop.borrow_mut().take_elapsed()? else {
      return Ok(false);
    };

    eval_returnable_script(&script, self, GLOBAL_FRAME).await?;
    Ok(true)
  }

  pub fn parse_script_caching(
    &mut self,
    src: &str,
  ) -> Result<Rc<(ScriptNode, String)>, ParseError> {
    if !self.parse_cache_script.contains(src) {
      let (node, rest) = parser::parse_script(src, parser::ParseMode::Script)?;
      self
        .parse_cache_script
        .put(src.to_string(), Rc::new((node, rest.to_string())));
    }
    Ok(self.parse_cache_script.get(src).unwrap().clone())
  }

  pub fn parse_expr_caching(&mut self, src: &str) -> Result<Rc<(ExprNode, String)>, ParseError> {
    if !self.parse_cache_expr.contains(src) {
      let (node, rest) = parser_expr::parse_expr(src)?;
      self
        .parse_cache_expr
        .put(src.to_string(), Rc::new((node, rest.to_string())));
    }
    Ok(self.parse_cache_expr.get(src).unwrap().clone())
  }

  pub fn get_variable(&self, frame: FrameId, name: &str) -> Option<&Value> {
    let mut cur_frame = frame;
    let mut cur_name = name;
    loop {
      match self.frame(cur_frame).get_binding(cur_name)? {
        Binding::Ref(ref_frame, ref_name) => {
          if *ref_frame == frame && ref_name == name {
            panic!("circular reference to {}", ref_name);
          }
          cur_frame = *ref_frame;
          cur_name = ref_name;
        }
        Binding::Val(v) => return Some(v),
      }
    }
  }

  pub fn set_variable(&mut self, frame: FrameId, name: &str, value: Value) {
    let mut cur_frame = frame;
    let mut cur_name = name;
    loop {
      match self.frame(cur_frame).get_binding(cur_name) {
        Some(Binding::Ref(ref_frame, ref_name)) => {
          if *ref_frame == frame && ref_name == name {
            panic!("circular reference to {}", ref_name);
          }
          cur_frame = *ref_frame;
          cur_name = ref_name;
        }
        Some(Binding::Val(_)) | None => {
          let name = cur_name.to_string();
          self
            .frame_mut(cur_frame)
            .set_binding(name.as_ref(), Binding::Val(value));
          return;
        }
      }
    }
  }

  pub fn ref_variable(
    &mut self,
    ref_frame: FrameId,
    ref_name: &str,
    target_frame: FrameId,
    target_name: &str,
  ) {
    self.frame_mut(ref_frame).set_binding(
      ref_name,
      Binding::Ref(target_frame, target_name.to_string()),
    );
  }
}

impl EvalFrame {
  pub fn new() -> EvalFrame {
    EvalFrame {
      caller: None,
      variables: HashMap::new(),
    }
  }

  pub fn new_from(frame: FrameId) -> EvalFrame {
    EvalFrame {
      caller: Some(frame),
      variables: HashMap::new(),
    }
  }

  pub fn get_binding(&self, name: &str) -> Option<&Binding> {
    self.variables.get(name)
  }

  pub fn get_binding_mut(&mut self, name: &str) -> Option<&mut Binding> {
    self.variables.get_mut(name)
  }

  pub fn set_binding(&mut self, name: &str, binding: Binding) {
    self.variables.insert(name.to_string(), binding);
  }
}
