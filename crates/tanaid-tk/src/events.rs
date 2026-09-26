use regex::Regex;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::LazyLock};
use tanaid::{
  eval::{GLOBAL_FRAME, eval_returnable_script},
  eval_error::EvalError,
  event_loop,
  parser::{self, ScriptNode},
};
use winit::{
  event::{KeyEvent, Modifiers},
  keyboard::{Key, ModifiersState},
};

pub type Tag = String;
pub type ScriptStr = String;
pub type EventBindingsData = HashMap<Tag, HashMap<Vec<TclEvent>, ScriptStr>>;

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

  pub fn get_binding(&self, tag: &Tag, sequence: Vec<TclEvent>) -> Option<&str> {
    self
      .bindings
      .get(tag)
      .and_then(|by_tag| by_tag.get(&sequence).map(|s| s.as_str()))
  }

  pub fn bind(&mut self, tag: Tag, sequence: Vec<TclEvent>, script_str: &str, append: bool) {
    let binding = self
      .bindings
      .entry(tag)
      .or_insert_with(|| HashMap::new())
      .entry(sequence);

    binding
      .and_modify(|v| {
        *v = if append {
          format!("{};\n{}", v, script_str)
        } else {
          script_str.to_string()
        };
      })
      .or_insert(script_str.to_string());
  }

  pub fn handle_modifiers(&mut self, mods: Modifiers) {
    self.modifiers = mods;
  }

  pub fn handle_key_event(
    &self,
    tag: Tag,
    key_event: KeyEvent,
    event_loop: Rc<RefCell<event_loop::EventLoop>>,
  ) -> Result<(), EvalError> {
    let event_type = match key_event.state {
      winit::event::ElementState::Pressed => TclEventType::KeyPress,
      winit::event::ElementState::Released => TclEventType::KeyRelease,
    };
    self.handle_key(
      tag,
      event_type,
      &key_detail(&key_event.logical_key),
      event_loop,
    )
  }

  fn handle_key(
    &self,
    tag: Tag,
    event_type: TclEventType,
    detail: &str,
    event_loop: Rc<RefCell<event_loop::EventLoop>>,
  ) -> Result<(), EvalError> {
    let mods = self.modifiers.state();
    let event_type = Some(event_type);

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
        // TODO: record and test against multi-key sequences
        if let Some(script_str) = self.get_binding(tag, vec![specificity.clone()]) {
          // TODO: template substitutions for key codes etc.
          // TODO: parse with caching
          let script =
            parser::parse(script_str).map_err(|e| EvalError::ScriptParseError(e.to_string()))?;

          event_loop
            .borrow_mut()
            .push_immediate(Box::new(TkEvent { script }));
          return Ok(());
        }
      }
    }

    Ok(())
  }
}

fn key_detail(key: &Key) -> TclEventDetail {
  key.to_text().unwrap_or("").to_string()
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

#[cfg(test)]
mod tests {
  use super::*;
  use winit::keyboard::NamedKey;

  fn bind(bindings: &mut EventBindings, tag: &str, sequence: &str) {
    let sequence = parse_sequence(sequence).unwrap();
    bindings.bind(tag.to_string(), sequence, "set x 1", false);
  }

  /// press a key on "." and return how many scripts were queued
  fn press(bindings: &EventBindings, detail: &str) -> usize {
    let event_loop = Rc::new(RefCell::new(event_loop::EventLoop::new()));
    bindings
      .handle_key(
        ".".to_string(),
        TclEventType::KeyPress,
        detail,
        Rc::clone(&event_loop),
      )
      .unwrap();
    event_loop.borrow().count_pending()
  }

  #[test]
  fn keypress_without_detail_matches_any_key() {
    let mut bindings = EventBindings::new();
    bind(&mut bindings, ".", "<KeyPress>");
    assert_eq!(press(&bindings, "a"), 1);
  }

  #[test]
  fn keypress_detail_matches_with_unbound_modifier_held() {
    let mut bindings = EventBindings::new();
    bind(&mut bindings, ".", "<KeyPress-A>");
    bindings.handle_modifiers(ModifiersState::SHIFT.into());
    assert_eq!(press(&bindings, "A"), 1);
  }

  #[test]
  fn widget_binding_does_not_suppress_all_binding() {
    let mut bindings = EventBindings::new();
    bind(&mut bindings, ".", "<KeyPress-w>");
    bind(&mut bindings, "all", "<KeyPress-w>");
    assert_eq!(press(&bindings, "w"), 2);
  }

  #[test]
  fn key_is_alias_for_keypress() {
    assert_eq!(
      parse_sequence("<Key-Left>").unwrap(),
      parse_sequence("<KeyPress-Left>").unwrap()
    );
  }

  #[test]
  fn named_keys_map_to_tk_keysyms() {
    assert_eq!(key_detail(&Key::Character("a".into())), "a");
    assert_eq!(key_detail(&Key::Named(NamedKey::ArrowUp)), "Up");
    assert_eq!(key_detail(&Key::Named(NamedKey::Enter)), "Return");
    assert_eq!(key_detail(&Key::Named(NamedKey::Space)), "space");
  }
}
