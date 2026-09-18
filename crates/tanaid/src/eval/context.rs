use super::Proc;
use super::event_loop::EventLoop;
use crate::eval::{EvalCmdResult, eval_returnable_script};
use crate::eval_error::EvalError;
use crate::event_loop::{Event, EventId, EventWait, EventWaiter};
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
pub const GLOBAL_FRAME: FrameId = 0;

pub type OutputSink = Rc<dyn Fn(&str) -> Result<(), EvalError>>;
pub type CommandHandler = dyn for<'a> Fn(
  &'a mut [Value],
  &'a mut EvalContext,
  FrameId,
) -> Pin<Box<dyn Future<Output = EvalCmdResult> + 'a>>;

pub struct EvalContext {
  procs: HashMap<String, Rc<Proc>>,
  commands: HashMap<String, Rc<CommandHandler>>,
  frame_id: usize,
  frames: HashMap<FrameId, EvalFrame>,
  pub event_loop: Rc<RefCell<EventLoop>>,

  clock_monotonic: Option<Rc<dyn Fn() -> Duration>>,
  clock_unixtime: Option<Rc<dyn Fn() -> Duration>>,
  sleep_ms:
    Option<Rc<dyn Fn(u64) -> Pin<Box<dyn Future<Output = Result<(), EvalError>>>> + 'static>>,

  parse_cache_script: LruCache<String, Rc<(ScriptNode, String)>>,
  parse_cache_expr: LruCache<String, Rc<(ExprNode, String)>>,

  pub(crate) stdout: OutputSink,
}

#[derive(Clone, Debug)]
pub struct EvalFrame {
  caller: Option<FrameId>,
  variables: HashMap<String, Binding>,
  variable_revs: HashMap<String, u64>,
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

      clock_monotonic: None,
      clock_unixtime: None,
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
    Fut: Future<Output = Result<(), EvalError>> + 'static,
  {
    self.sleep_ms = Some(Rc::new(move |ms: u64| Box::pin(f(ms))));
    self
  }

  pub fn with_clock_monotonic(mut self, f: impl Fn() -> Duration + 'static) -> Self {
    let clock: Rc<dyn Fn() -> Duration> = Rc::new(f);
    self.clock_monotonic = Some(Rc::clone(&clock));
    self.event_loop.borrow_mut().clock_monotonic = Some(Rc::clone(&clock));
    self
  }

  pub fn with_clock_unixtime(mut self, f: impl Fn() -> Duration + 'static) -> Self {
    self.clock_unixtime = Some(Rc::new(f));
    self
  }

  #[cfg(not(target_family = "wasm"))]
  pub fn with_std_time(self) -> Self {
    let start = std::time::Instant::now();
    self
      .with_sleep_ms(async |ms| {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(())
      })
      .with_clock_monotonic(move || std::time::Instant::now().duration_since(start))
      .with_clock_unixtime(|| {
        std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .expect("system clock set before UNIX_EPOCH")
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
  ) -> Result<EventId, EvalError> {
    let event = TimerEvent::new(timer_script);
    let deadline = self.clock_monotonic()? + Duration::from_millis(delay_ms);
    return Ok(
      self
        .event_loop
        .borrow_mut()
        .push_scheduled(Box::new(event), deadline),
    );
  }

  pub fn cancel_timer(&mut self, timer_id: EventId) -> Result<(), EvalError> {
    self.event_loop.borrow_mut().remove(timer_id);
    Ok(())
  }

  pub async fn sleep_ms(&self, ms: u64) -> Result<(), EvalError> {
    let sleep_ms = self.sleep_ms.as_ref().ok_or_else(|| {
      EvalError::Generic(
        "environment does not support timers (missing callback_sleep_ms)".to_string(),
      )
    })?;
    sleep_ms(ms).await
  }

  pub fn count_pending_events(&self) -> usize {
    self.event_loop.borrow().count_pending()
  }

  pub fn next_event_wait(&self) -> Result<EventWait, EvalError> {
    self.event_loop.borrow_mut().next_wait()
  }

  pub async fn wait_for_event(&self) -> Result<(), EvalError> {
    let wait = self.next_event_wait()?;
    match wait {
      EventWait::Ready => {}
      EventWait::Delay(delay) => {
        self.sleep_ms(delay.as_millis() as u64).await?;
      }
      EventWait::Idle => EventWaiter::wait(Rc::clone(&self.event_loop)).await,
    }
    Ok(())
  }

  pub fn clock_monotonic(&self) -> Result<Duration, EvalError> {
    Ok(self.clock_monotonic.as_ref().ok_or_else(|| {
      EvalError::Generic("Context missing clock_monotonic".to_string())
    })?())
  }

  pub fn clock_unixtime(&self) -> Result<Duration, EvalError> {
    Ok(self.clock_unixtime.as_ref().ok_or_else(|| {
      EvalError::Generic("Context missing clock_unixtime".to_string())
    })?())
  }

  pub async fn poll_event(&mut self) -> Result<bool, EvalError> {
    let Some(event) = self.event_loop.borrow_mut().take_ready()? else {
      return Ok(false);
    };

    event.dispatch(self).await?;
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

  pub fn get_variable_rev(&self, frame: FrameId, name: &str) -> u64 {
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
        Some(Binding::Val(_)) => return self.frame(cur_frame).get_rev(cur_name),
        None => return 0,
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
      variable_revs: HashMap::new(),
    }
  }

  pub fn new_from(frame: FrameId) -> EvalFrame {
    EvalFrame {
      caller: Some(frame),
      variables: HashMap::new(),
      variable_revs: HashMap::new(),
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

    self
      .variable_revs
      .entry(name.to_string())
      .and_modify(|v| *v += 1)
      .or_insert(1);
  }

  /// monotonically increasing counter that increases when variable is written
  pub fn get_rev(&self, name: &str) -> u64 {
    *self.variable_revs.get(name).unwrap_or(&0)
  }
}

// an EventLoop event that simply runs its script (on timer fire)
pub struct TimerEvent {
  script: ScriptNode,
}

impl TimerEvent {
  fn new(script: ScriptNode) -> Self {
    Self { script }
  }
}

impl Event for TimerEvent {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut EvalContext,
  ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>> {
    Box::pin(async move {
      // TODO: bgerror (don't abort event loops)
      eval_returnable_script(&self.script, ctx, GLOBAL_FRAME).await?;
      Ok(())
    })
  }
}
