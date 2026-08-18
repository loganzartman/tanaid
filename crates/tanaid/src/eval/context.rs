use super::Proc;
use crate::eval::eval_returnable_script;
use crate::eval_error::EvalError;
use crate::parser::{self, ParseError, ScriptNode};
use crate::parser_expr::{self, ExprNode};
use crate::value::Value;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::rc::Rc;

pub type FrameId = usize;
pub(crate) const GLOBAL_FRAME: FrameId = 0;

pub type OutputSink = Rc<dyn Fn(&str) -> Result<(), EvalError>>;
pub type CommandHandler =
  Rc<dyn Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError>>;

#[derive(Clone)]
pub struct EvalContext {
  procs: HashMap<String, Rc<Proc>>,
  commands: HashMap<String, CommandHandler>,
  frames: Vec<EvalFrame>,

  timer_id: usize,
  pending_timers: HashMap<usize, ScriptNode>,
  queued_timer_actions: Vec<TimerAction>,

  parse_cache_script: LruCache<String, Rc<(ScriptNode, String)>>,
  parse_cache_expr: LruCache<String, Rc<(ExprNode, String)>>,

  pub(crate) stdout: OutputSink,
}

#[derive(Clone, Debug)]
pub struct EvalFrame {
  #[expect(dead_code, reason = "populated by push_frame; not read back yet")]
  caller: Option<FrameId>,
  variables: HashMap<String, Binding>,
}

#[derive(Clone, Debug)]
pub enum Binding {
  Val(Value),
  Ref(FrameId, String),
}

#[derive(Clone, Debug)]
pub enum TimerAction {
  Start { timer_id: usize, delay_ms: u64 },
  Cancel { timer_id: usize },
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
      frames: vec![EvalFrame::new()],

      timer_id: 0,
      pending_timers: HashMap::new(),
      queued_timer_actions: Vec::new(),

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

  pub fn with_stdout(mut self, stdout: OutputSink) -> Self {
    self.stdout = stdout;
    self
  }

  pub fn frame(&self, id: FrameId) -> &EvalFrame {
    self.frames.get(id).unwrap()
  }

  pub fn frame_mut(&mut self, id: FrameId) -> &mut EvalFrame {
    self.frames.get_mut(id).unwrap()
  }

  pub fn run_with_frame<R>(
    &mut self,
    calling_frame: FrameId,
    f: impl FnOnce(&mut EvalContext, FrameId) -> R,
  ) -> R {
    let next_id = self.frames.len();
    self.frames.push(EvalFrame::new_from(calling_frame));
    let result = f(self, next_id);
    self.frames.pop();
    result
  }

  pub fn get_command(&self, name: &str) -> Option<CommandHandler> {
    self.commands.get(name).cloned()
  }

  pub fn register_command(&mut self, name: &str, handler: CommandHandler) {
    self.commands.insert(name.to_string(), handler.clone());
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

  pub fn take_timer_actions(&mut self) -> Vec<TimerAction> {
    return std::mem::take(&mut self.queued_timer_actions);
  }

  pub fn start_timer(
    &mut self,
    timer_script: &ScriptNode,
    delay_ms: u64,
  ) -> Result<usize, EvalError> {
    let timer_id = self.timer_id;
    self.timer_id = self.timer_id.strict_add(1);
    self.pending_timers.insert(timer_id, timer_script.clone());
    self
      .queued_timer_actions
      .push(TimerAction::Start { timer_id, delay_ms });
    return Ok(timer_id);
  }

  pub fn cancel_timer(&mut self, timer_id: usize) -> Result<bool, EvalError> {
    if self.pending_timers.remove(&timer_id).is_none() {
      return Ok(false);
    }
    self
      .queued_timer_actions
      .push(TimerAction::Cancel { timer_id });
    Ok(true)
  }

  pub fn fire_timer(&mut self, timer_id: usize) -> Result<Value, EvalError> {
    let Some(timer_script) = self.pending_timers.remove(&timer_id) else {
      return Err(EvalError::Generic(format!(
        "internal error: tried to fire nonexistent timer: {}",
        timer_id
      )));
    };

    eval_returnable_script(&timer_script, self, GLOBAL_FRAME)
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
