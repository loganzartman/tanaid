use super::Proc;
use crate::parser::{self, ParseError, ScriptNode};
use crate::parser_expr::{self, ExprNode};
use crate::value::Value;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::rc::Rc;

pub type FrameId = usize;
pub(crate) const GLOBAL_FRAME: FrameId = 0;

#[derive(Clone, Debug)]
pub struct EvalContext {
  procs: HashMap<String, Rc<Proc>>,
  frames: Vec<EvalFrame>,
  parse_cache_script: LruCache<String, Rc<(ScriptNode, String)>>,
  parse_cache_expr: LruCache<String, Rc<(ExprNode, String)>>,
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

impl EvalContext {
  pub fn new() -> EvalContext {
    EvalContext {
      procs: HashMap::new(),
      frames: vec![EvalFrame::new()],
      parse_cache_script: LruCache::new(NonZeroUsize::new(1024).unwrap()),
      parse_cache_expr: LruCache::new(NonZeroUsize::new(1024).unwrap()),
    }
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

  pub fn get_proc(&self, name: &str) -> Option<Rc<Proc>> {
    self.procs.get(name).cloned()
  }

  pub fn set_proc(&mut self, name: &str, proc: Proc) {
    self.procs.insert(name.to_string(), Rc::new(proc));
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
