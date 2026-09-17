use regex::Regex;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::LazyLock};
use tanaid::{
  eval::{GLOBAL_FRAME, eval_returnable_script},
  eval_error::EvalError,
  event_loop,
  parser::{ParseError, ScriptNode},
};
use winit::{
  event::{KeyEvent, Modifiers},
  keyboard::{Key, ModifiersState},
  platform::modifier_supplement::KeyEventExtModifierSupplement,
};

pub type Tag = String;
pub type EventBindingsData = HashMap<Tag, HashMap<Vec<Event>, ScriptNode>>;

pub struct EventBindings {
  bindings: EventBindingsData,
  modifiers: Modifiers,
}

#[derive(PartialEq, Eq, Hash)]
pub enum Event {
  Key(KeyType, Option<Key>, Option<ModifiersState>),
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum KeyType {
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

  pub fn get_binding(&self, tag: &Tag, sequence: Vec<Event>) -> Option<&ScriptNode> {
    self
      .bindings
      .get(tag)
      .and_then(|by_tag| by_tag.get(&sequence))
  }

  pub fn bind(&mut self, tag: Tag, sequence: Vec<Event>, script: &ScriptNode, append: bool) {
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
    let key = key_event.key_without_modifiers();
    let mods = self.modifiers.state();
    let ktype = match key_event.state {
      winit::event::ElementState::Pressed => KeyType::KeyPress,
      winit::event::ElementState::Released => KeyType::KeyRelease,
    };

    let tags: &[String] = if tag == "all" {
      &[tag]
    } else {
      &[tag, "all".to_string()]
    };

    for tag in tags {
      for specificity in [
        Event::Key(ktype.clone(), Some(key.clone()), Some(mods.clone())),
        Event::Key(ktype.clone(), Some(key.clone()), None),
        Event::Key(ktype.clone(), None, None),
      ] {
        if let Some(script) = self.get_binding(tag, vec![specificity]) {
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

pub fn parse_sequence(mut raw: &str) -> Result<Vec<Event>, ParseError> {
  static RE_CHAR: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[^<>]").unwrap());
  static RE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^<(?<pattern>[^<>]+)>").unwrap());
  static RE_VIRTUAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^<<(?<name>[^<>]+)>>").unwrap());

  let mut seq: Vec<Event> = vec![];

  while !raw.is_empty() {
    if let Some(caps) = RE_CHAR.captures(raw) {
      let m = caps.get_match().as_str();
      seq.push(Event::Key(KeyType::KeyPress, Some(parse_key(m)), None));
      raw = &raw[m.len()..];
    } else if let Some(_) = RE_PATTERN.captures(raw) {
      return Err(ParseError::Generic(
        "event patterns not implemented".to_string(),
      ));
    } else if let Some(_) = RE_VIRTUAL.captures(raw) {
      return Err(ParseError::Generic(
        "virtual events not implemented".to_string(),
      ));
    } else {
      return Err(ParseError::Generic("invalid event sequence".to_string()));
    }
  }

  Ok(seq)
}

pub fn parse_key(raw: &str) -> Key {
  // TODO
  Key::Character(raw.into())
}
