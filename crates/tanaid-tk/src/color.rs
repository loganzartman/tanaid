/// A straight-alpha RGBA color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
  pub r: f32,
  pub g: f32,
  pub b: f32,
  pub a: f32,
}

/// The subset of X11 color names Tk scripts reach for most often.
const NAMED_COLORS: &[(&str, u32)] = &[
  ("black", 0x000000),
  ("blue", 0x0000ff),
  ("brown", 0xa52a2a),
  ("cyan", 0x00ffff),
  ("gray", 0xbebebe),
  ("green", 0x00ff00),
  ("grey", 0xbebebe),
  ("magenta", 0xff00ff),
  ("navy", 0x000080),
  ("orange", 0xffa500),
  ("pink", 0xffc0cb),
  ("purple", 0xa020f0),
  ("red", 0xff0000),
  ("white", 0xffffff),
  ("yellow", 0xffff00),
];

impl Color {
  pub const fn rgb(rgb: u32) -> Color {
    Color {
      r: ((rgb >> 16) & 0xff) as f32 / 255.,
      g: ((rgb >> 8) & 0xff) as f32 / 255.,
      b: (rgb & 0xff) as f32 / 255.,
      a: 1.,
    }
  }

  /// Parses a Tk color: either `#rgb`/`#rrggbb`, or one of [`NAMED_COLORS`].
  pub fn parse(spec: &str) -> Option<Color> {
    if let Some(digits) = spec.strip_prefix('#') {
      let widen = |digit: u32| digit * 0x11;
      return match digits.len() {
        3 => {
          let value = u32::from_str_radix(digits, 16).ok()?;
          Some(Color::rgb(
            (widen((value >> 8) & 0xf) << 16)
              | (widen((value >> 4) & 0xf) << 8)
              | widen(value & 0xf),
          ))
        }
        6 => Some(Color::rgb(u32::from_str_radix(digits, 16).ok()?)),
        _ => None,
      };
    }

    let lowercased = spec.to_ascii_lowercase();
    NAMED_COLORS
      .iter()
      .find(|(name, _)| *name == lowercased)
      .map(|(_, rgb)| Color::rgb(*rgb))
  }

  pub fn to_array(self) -> [f32; 4] {
    [self.r, self.g, self.b, self.a]
  }

  /// Color specs are sRGB encoded, but an sRGB render target expects linear
  /// values and encodes them itself.
  pub fn to_linear_array(self) -> [f32; 4] {
    fn to_linear(component: f32) -> f32 {
      if component <= 0.04045 {
        component / 12.92
      } else {
        ((component + 0.055) / 1.055).powf(2.4)
      }
    }

    [
      to_linear(self.r),
      to_linear(self.g),
      to_linear(self.b),
      self.a,
    ]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_named_colors() {
    assert_eq!(Color::parse("red"), Some(Color::rgb(0xff0000)));
    assert_eq!(Color::parse("White"), Some(Color::rgb(0xffffff)));
    assert_eq!(Color::parse("chartreuse"), None);
  }

  #[test]
  fn parses_hex_colors() {
    assert_eq!(Color::parse("#e2725b"), Some(Color::rgb(0xe2725b)));
    assert_eq!(Color::parse("#abc"), Some(Color::rgb(0xaabbcc)));
    assert_eq!(Color::parse("#abcd"), None);
    assert_eq!(Color::parse("#zzzzzz"), None);
  }
}
