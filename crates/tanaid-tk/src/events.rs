use regex::Regex;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::LazyLock};
use tanaid::{
  eval::{GLOBAL_FRAME, eval_returnable_script},
  eval_error::EvalError,
  event_loop,
  parser::ScriptNode,
};
use winit::{
  event::{KeyEvent, Modifiers},
  keyboard::ModifiersState,
};

pub type Tag = String;
pub type EventBindingsData = HashMap<Tag, HashMap<Vec<TclEvent>, ScriptNode>>;

pub struct EventBindings {
  bindings: EventBindingsData,
  modifiers: Modifiers,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TclEvent {
  event_type: Option<TclEventType>,
  modifiers: Option<ModifiersState>,
  detail: Option<TclEventDetail>,
}
type TclEventDetail = String;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum TclEventType {
  KeyPress,
  KeyRelease,
}

impl EventBindings {
  pub fn new() -> Self {
    EventBindings {
      bindings: HashMap::new(),
      modifiers: Modifiers::default(),
    }
  }

  pub fn get_binding(&self, tag: &Tag, sequence: Vec<TclEvent>) -> Option<&ScriptNode> {
    self
      .bindings
      .get(tag)
      .and_then(|by_tag| by_tag.get(&sequence))
  }

  pub fn bind(&mut self, tag: Tag, sequence: Vec<TclEvent>, script: &ScriptNode, append: bool) {
    let binding = self
      .bindings
      .entry(tag)
      .or_insert_with(|| HashMap::new())
      .entry(sequence);

    binding
      .and_modify(|v| {
        *v = if append {
          ScriptNode::concat(v, script)
        } else {
          script.clone()
        };
      })
      .or_insert(script.clone());
  }

  pub fn handle_modifiers(&mut self, mods: Modifiers) {
    self.modifiers = mods;
  }

  pub fn handle_key_event(
    &self,
    tag: Tag,
    key_event: KeyEvent,
    event_loop: Rc<RefCell<event_loop::EventLoop>>,
  ) {
    let detail = key_event.logical_key.to_text().unwrap_or("").to_string();
    let mods = self.modifiers.state();
    let event_type = match key_event.state {
      winit::event::ElementState::Pressed => Some(TclEventType::KeyPress),
      winit::event::ElementState::Released => Some(TclEventType::KeyRelease),
    };

    let tags: &[String] = if tag == "all" {
      &[tag]
    } else {
      &[tag, "all".to_string()]
    };

    let specificities = &[
      TclEvent {
        event_type: event_type.clone(),
        modifiers: Some(mods),
        detail: Some(detail.to_string()),
      },
      TclEvent {
        event_type: event_type.clone(),
        modifiers: None,
        detail: Some(detail.to_string()),
      },
      TclEvent {
        event_type: event_type.clone(),
        modifiers: None,
        detail: None,
      },
      TclEvent {
        event_type: None,
        modifiers: Some(mods),
        detail: Some(detail.to_string()),
      },
      TclEvent {
        event_type: None,
        modifiers: None,
        detail: Some(detail.to_string()),
      },
    ];

    for tag in tags {
      for specificity in specificities {
        if let Some(script) = self.get_binding(tag, vec![specificity.clone()]) {
          event_loop.borrow_mut().push_immediate(Box::new(TkEvent {
            script: script.clone(),
          }));
          return;
        }
      }
    }
  }
}

struct TkEvent {
  script: ScriptNode,
}

impl tanaid::event_loop::Event for TkEvent {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut tanaid::eval::EvalContext,
  ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>> {
    Box::pin(async move {
      // TODO: bgerror (don't abort event loops)
      eval_returnable_script(&self.script, ctx, GLOBAL_FRAME).await?;
      Ok(())
    })
  }
}

pub fn parse_sequence(mut raw: &str) -> Result<Vec<TclEvent>, EvalError> {
  static RE_CHAR: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[^<>]").unwrap());
  static RE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^<(?<pattern>[^<>]+)>").unwrap());
  static RE_VIRTUAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^<<(?<name>[^<>]+)>>").unwrap());

  let mut seq: Vec<TclEvent> = vec![];

  while !raw.is_empty() {
    if let Some(caps) = RE_CHAR.captures(raw) {
      let m = caps.get_match().as_str();
      seq.push(TclEvent {
        event_type: None,
        detail: Some(parse_key(m)),
        modifiers: None,
      });
      raw = &raw[caps.get_match().len()..];
    } else if let Some(caps) = RE_PATTERN.captures(raw) {
      let m = caps
        .get(1)
        .expect("event pattern should have required group")
        .as_str();
      seq.push(parse_sequence_pattern(m)?);
      raw = &raw[caps.get_match().len()..];
    } else if let Some(_) = RE_VIRTUAL.captures(raw) {
      return Err(EvalError::Generic(
        "virtual events not implemented".to_string(),
      ));
    } else {
      return Err(EvalError::Generic("invalid event sequence".to_string()));
    }
  }

  Ok(seq)
}

fn parse_sequence_pattern(raw: &str) -> Result<TclEvent, EvalError> {
  let parts_vec = raw.split("-").collect::<Vec<_>>();
  let mut parts = parts_vec.iter().peekable();

  let mut mods = ModifiersState::empty();
  while let Some(&&part) = parts.peek() {
    match part {
      "Control" => mods = mods.union(ModifiersState::CONTROL),
      "Alt" => mods = mods.union(ModifiersState::ALT),
      "Shift" => mods = mods.union(ModifiersState::SHIFT),
      "Extended" => mods = mods.union(ModifiersState::SUPER),
      _ => break,
    }
    parts.next();
  }

  let mut event_type: Option<TclEventType> = None;
  let mut detail: Option<TclEventDetail> = None;

  match parts.next() {
    Some(&"KeyPress") => event_type = Some(TclEventType::KeyPress),
    Some(&"KeyRelease") => event_type = Some(TclEventType::KeyRelease),
    Some(&d) => detail = Some(d.to_string()),
    None => {
      return Err(EvalError::ArgumentError(
        "bind requires event type or detail".to_string(),
      ));
    }
  }

  if event_type.is_some() && detail.is_none() {
    if let Some(&part) = parts.next() {
      detail = Some(part.to_string());
    }
  }

  if let Some(_) = parts.next() {
    return Err(EvalError::ArgumentError(
      "bind: extra characters after detail".to_string(),
    ));
  }

  let e = TclEvent {
    event_type,
    modifiers: Some(mods),
    detail,
  };
  Ok(e)
}

fn parse_key(raw: &str) -> TclEventDetail {
  // TODO
  raw.to_string()
}
