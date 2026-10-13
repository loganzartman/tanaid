use serde::{
  Deserialize, Serialize,
  de::value::{Error, StrDeserializer},
};
use tsify::Tsify;
use winit::keyboard::{Key, KeyLocation, ModifiersState, NamedKey, NativeKey};

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TclModifierKeys {
  shift: bool,
  ctrl: bool,
  alt: bool,
  meta: bool,
}

/// Returns the winit key for a `KeyboardEvent.key` value.
///
/// `NamedKey` variants are named after the DOM key values, so a named key deserializes straight
/// into its variant. Only `"Meta"` and `" "` are spelled differently. Anything that isn't a
/// named key is the text the key produces.
pub(crate) fn key_from_js(key: &str) -> Key {
  match key {
    "Unidentified" => Key::Unidentified(NativeKey::Web(key.into())),
    "Dead" => Key::Dead(None),
    "Meta" => Key::Named(NamedKey::Super),
    " " => Key::Named(NamedKey::Space),
    _ => NamedKey::deserialize(StrDeserializer::<Error>::new(key))
      .map_or_else(|_| Key::Character(key.into()), Key::Named),
  }
}

/// Returns the winit key location for a `KeyboardEvent.location` value.
pub(crate) fn key_location_from_js(location: u32) -> KeyLocation {
  match location {
    1 => KeyLocation::Left,
    2 => KeyLocation::Right,
    3 => KeyLocation::Numpad,
    _ => KeyLocation::Standard,
  }
}

pub(crate) fn modifiers_from_js(modifiers: TclModifierKeys) -> ModifiersState {
  let mut state = ModifiersState::empty();
  state.set(ModifiersState::SHIFT, modifiers.shift);
  state.set(ModifiersState::CONTROL, modifiers.ctrl);
  state.set(ModifiersState::ALT, modifiers.alt);
  state.set(ModifiersState::SUPER, modifiers.meta);
  state
}
