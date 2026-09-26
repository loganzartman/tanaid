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
  keyboard::ModifiersState,
};

use crate::keysym::key_detail;

pub type Tag = String;
pub type ScriptStr = String;
pub type EventBindingsData = HashMap<Tag, HashMap<Vec<TclEvent>, ScriptStr>>;

/// Implements Tcl/Tk event binding logic.
///
/// Binding an event associates a script with a given *tag* and *event sequence*.
/// - the tag is the Tk widget that listens for the event, or "all"
/// - the sequence is one or more (TODO) events in short succession; for example, pressing Ctrl-x then Ctrl-f
/// - each event consists of
///   - event type: KeyPress, KeyRelease, etc.
///   - modifiers: Set of active modifier keys: Control, Alt, etc.
///   - detail: Keysym (e.g. Z, Left, etc.) or mouse button
///
/// An incoming event always has a detail (because, e.g., some key must have been pressed), but
/// an event binding may specify no detail, meaning to match, e.g., any key.
///
/// An event *matches* a binding if that binding:
/// - has the same event type, AND
/// - has a subset of the event's modifiers, AND
/// - has the same detail or no detail
///
/// For a unique combination of tag and event sequence, there is at most one bound script.
/// When binding, the user chooses whether to replace the script or append to it.
///
/// When dispatching an event, at most two scripts run:
/// 1. The most specific matching binding, if any, for the tag that received the event, AND
/// 2. The most specific matching binding, if any, for the tag "all".
///
/// "Most-specific" binding means, in order:
/// 1. Has a detail (<KeyPress-f> beats <KeyPress>)
/// 2. Longest event sequence (e.g. <Control-x><Control-f> beats <Control-f>)
/// 3. Larger modifier set (<Control-Alt-a> beats <Alt-a>); this differs from Tk.
pub struct EventBindings {
  bindings: EventBindingsData,
  modifiers: Modifiers,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TclEvent {
  event_type: TclEventType,
  modifiers: ModifiersState,
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
      &key_detail(&key_event.logical_key, key_event.location),
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
    // TODO: record multi-key sequences

    let mods = self.modifiers.state();

    let tags: &[String] = if tag == "all" {
      &[tag]
    } else {
      &[tag, "all".to_string()]
    };

    let event = TclEvent {
      event_type: event_type,
      modifiers: mods,
      detail: Some(detail.to_string()),
    };

    for tag in tags {
      let Some(bindings) = self.bindings.get(tag) else {
        continue;
      };

      // TODO: prepend actual history
      let history_seq = std::slice::from_ref(&event);

      for (binding_seq, script_str) in bindings {
        if history_matches(history_seq, binding_seq) {
          // TODO: template substitutions for key codes etc.
          // TODO: parse with caching
          let script =
            parser::parse(script_str).map_err(|e| EvalError::ScriptParseError(e.to_string()))?;

          event_loop
            .borrow_mut()
            .push_immediate(Box::new(TkEvent { script }));
          break;
        }
      }
    }

    Ok(())
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
        event_type: TclEventType::KeyPress,
        detail: Some(parse_key(m)),
        modifiers: ModifiersState::empty(),
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

  let mut modifiers = ModifiersState::empty();
  while let Some(&&part) = parts.peek() {
    match part {
      "Control" => modifiers = modifiers.union(ModifiersState::CONTROL),
      "Alt" => modifiers = modifiers.union(ModifiersState::ALT),
      "Shift" => modifiers = modifiers.union(ModifiersState::SHIFT),
      "Extended" => modifiers = modifiers.union(ModifiersState::SUPER),
      _ => break,
    }
    parts.next();
  }

  let mut event_type: Option<TclEventType> = None;
  let mut detail: Option<TclEventDetail> = None;

  match parts.next() {
    Some(&"Key") => event_type = Some(TclEventType::KeyPress),
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
    event_type: event_type.unwrap_or(TclEventType::KeyPress),
    modifiers,
    detail,
  };
  Ok(e)
}

fn parse_key(raw: &str) -> TclEventDetail {
  // TODO
  raw.to_string()
}

fn history_matches(history_seq: &[TclEvent], binding_seq: &[TclEvent]) -> bool {
  for (h, b) in history_seq.iter().rev().zip(binding_seq.iter().rev()) {
    if h.event_type != b.event_type {
      return false;
    }
    if !h.modifiers.contains(b.modifiers) {
      return false;
    }
    if b.modifiers > h.modifiers {
      return false;
    }
  }
  return true;
}

#[cfg(test)]
mod tests {
  use super::*;

  fn bind(bindings: &mut EventBindings, tag: &str, sequence: &str) {
    let sequence = parse_sequence(sequence).unwrap();
    bindings.bind(tag.to_string(), sequence, "set x 1", false);
  }

  /// send a key event to "." and return how many scripts were queued
  fn key(bindings: &EventBindings, event_type: TclEventType, detail: &str) -> usize {
    let event_loop = Rc::new(RefCell::new(event_loop::EventLoop::new()));
    bindings
      .handle_key(".".to_string(), event_type, detail, Rc::clone(&event_loop))
      .unwrap();
    event_loop.borrow().count_pending()
  }

  fn press(bindings: &EventBindings, detail: &str) -> usize {
    key(bindings, TclEventType::KeyPress, detail)
  }

  #[test]
  fn bare_char_binds_keypress_only() {
    let mut bindings = EventBindings::new();
    bind(&mut bindings, ".", "a");
    assert_eq!(press(&bindings, "a"), 1);
    assert_eq!(key(&bindings, TclEventType::KeyRelease, "a"), 0);
  }

  #[test]
  fn keypress_without_detail_matches_any_key() {
    let mut bindings = EventBindings::new();
    println!("KP {:?}", parse_sequence("<KeyPress>"));
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
}
