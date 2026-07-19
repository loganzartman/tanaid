use regex::Regex;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::mem::take;
use std::sync::LazyLock;

#[derive(Debug)]
pub enum ParseError {
  Generic(String),
  /// more input may resolve this error
  Continuable(String),
  /// internal error, should not happen
  Internal(String),
}

impl Display for ParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ParseError::Generic(s) => write!(f, "[ParseError]: {}", s),
      ParseError::Continuable(s) => write!(f, "[ParseError]: {}", s),
      ParseError::Internal(s) => write!(f, "[ParseError] internal: {}", s),
    }
  }
}

impl std::error::Error for ParseError {}

#[derive(PartialEq, Clone, Debug)]
pub struct ScriptNode {
  pub commands: Vec<CommandNode>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct CommandNode {
  pub words: Vec<WordNode>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct WordNode {
  pub parts: Vec<WordPart>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum WordPart {
  BareLiteral(String),
  BracedLiteral(String),
  Quoted(Vec<WordPart>),
  VarSub(String),
  VarIndex(String, String),
  BracedSub(String),
  BracedIndex(String, String),
  CommandSub(String),
}

impl WordNode {
  pub fn only(part: WordPart) -> WordNode {
    WordNode { parts: vec![part] }
  }
}

impl Display for ScriptNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    for n in self.commands.iter() {
      write!(f, "{};\n", n.to_string())?;
    }
    Ok(())
  }
}

impl Display for CommandNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    for (i, n) in self.words.iter().enumerate() {
      if i > 0 {
        write!(f, " ")?;
      }
      write!(f, "{}", n.to_string())?;
    }
    Ok(())
  }
}

impl Display for WordNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    for p in self.parts.iter() {
      write!(f, "{}", p)?;
    }
    Ok(())
  }
}

impl Display for WordPart {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    use WordPart::*;
    match self {
      BareLiteral(s) => write!(f, "{}", s),
      BracedLiteral(s) => write!(f, "{{{}}}", s),
      Quoted(s) => {
        let escape = |part: &WordPart| part.to_string().replace("\"", "\\\"");
        for part in s {
          write!(f, "\"{}\"", escape(part))?;
        }
        Ok(())
      }
      VarSub(v) => write!(f, "${}", v),
      VarIndex(v, i) => write!(f, "${}({})", v, i),
      BracedSub(v) => write!(f, "${{{}}}", v),
      BracedIndex(v, i) => write!(f, "${{{}({})}}", v, i),
      CommandSub(c) => write!(f, "[{}]", c.to_string()),
    }
  }
}

pub fn parse(src: &str) -> Result<ScriptNode, ParseError> {
  let (script_node, _) = parse_script(src, ParseMode::Script)?;
  return Ok(script_node);
}

pub(crate) enum ParseMode {
  Script,
  CommandSub,
}

pub(crate) fn parse_script(
  mut src: &str,
  mode: ParseMode,
) -> Result<(ScriptNode, &str), ParseError> {
  let mut commands: Vec<CommandNode> = vec![];

  while !src.is_empty() {
    match parse_command(src) {
      Ok((command, rest)) => {
        commands.push(command);
        src = rest;
      }
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => return Err(ParseError::Generic("expected command".to_string())),
    }

    match parse_ws(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => {}
    }

    match parse_command_sep(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => break,
    }

    // eat whitespace
    match parse_ws_or_command_sep(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => {}
    }
  }

  Ok((ScriptNode { commands }, src))
}

fn parse_command(mut src: &str) -> Result<(CommandNode, &str), ParseError> {
  // eat whitespace
  match parse_ws_or_command_sep(src) {
    Ok((_, rest)) => src = rest,
    Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
    Err(_) => {}
  }

  // required: first word (command name)
  let (name, rest) = match parse_word(src) {
    Ok(result) => result,
    Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
    Err(err) => {
      return Err(ParseError::Generic(format!(
        "expected command name\ncaused by: {}",
        err
      )));
    }
  };
  let mut words: Vec<WordNode> = vec![name];
  src = rest;

  // collect additional words
  while !src.is_empty() {
    // required whitespace separator
    let (_, rest) = match parse_ws(src) {
      Ok(result) => result,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => break,
    };
    src = rest;

    // word
    let (word, rest) = match parse_word(src) {
      Ok(result) => result,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => break,
    };
    words.push(word);
    src = rest;
  }

  Ok((CommandNode { words }, src))
}

pub fn parse_list(mut src: &str) -> Result<(Vec<String>, &str), ParseError> {
  let mut items: Vec<String> = vec![];
  let mut first = true;

  while !src.is_empty() {
    // required whitespace separator
    match parse_ws(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) if !first => break,
      Err(_) => {}
    }
    first = false;

    match parse_list_element_bare(src) {
      Ok((str, rest)) => {
        items.push(str);
        src = rest;
      }
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => break,
    }
  }

  Ok((items, src))
}

fn parse_list_element_bare(src: &str) -> Result<(String, &str), ParseError> {
  static RE_BARE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[^{}"\s]+"#).unwrap());

  if let Some(captures) = RE_BARE.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic(
      "expected bare list element".to_string(),
    ))
  }
}

pub(crate) fn parse_word(mut src: &str) -> Result<(WordNode, &str), ParseError> {
  // trim leading whitespace
  match parse_ws(src) {
    Ok((_, rest)) => src = rest,
    Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
    Err(_) => {}
  }

  enum State {
    START,
    BARE,
    VARSUB,
  }
  use State::*;
  let mut state = START;

  let mut parts: Vec<WordPart> = vec![];
  let mut part_buffer = String::new();

  loop {
    // synthetic null terminator at end-of-input makes it easier to handle termination
    let ch = src.chars().next().unwrap_or('\0');

    match state {
      START => match ch {
        '{' => {
          let (s, rest) = parse_curly_braced_string(src)?;
          return Ok((
            WordNode {
              parts: vec![WordPart::BracedLiteral(s)],
            },
            rest,
          ));
        }
        '"' => todo!(),
        '\0' => return Err(ParseError::Generic("expected word".to_string())),
        _ => {
          state = BARE;
        }
      },

      BARE => match ch {
        ' ' | '\t' | '\n' | '\r' | ';' | '\0' => {
          // flush
          if !part_buffer.is_empty() {
            parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
          }

          break;
        }
        '$' => {
          // flush
          if !part_buffer.is_empty() {
            parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
          }

          state = VARSUB;
          src = &src[1..];
        }
        '[' => {
          // flush
          if !part_buffer.is_empty() {
            parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
          }

          let (s, rest) = todo!();
          parts.push(WordPart::CommandSub(s));
          src = rest;
        }
        '\\' => {
          let (escaped_ch, rest) = parse_backslash_escape(src)?;
          part_buffer.push(escaped_ch);
          src = rest;
        }
        _ => {
          part_buffer.push(ch);
          src = &src[ch.len_utf8()..];
        }
      },

      VARSUB => match ch {
        ' ' | '\t' | '\n' | '\r' | ';' | '$' | '[' | '\0' => {
          // flush
          if !part_buffer.is_empty() {
            parts.push(WordPart::VarSub(take(&mut part_buffer)));
          } else {
            parts.push(WordPart::BareLiteral("$".to_string()));
          }

          state = BARE;
        }
        '\\' => {
          if let Some('$') = src.chars().nth(1) {
            part_buffer.push_str(r"\$");
            src = &src[2..];
          } else {
            part_buffer.push('\\');
            src = &src[1..];
          }
        }
        _ => {
          part_buffer.push(ch);
          src = &src[ch.len_utf8()..];
        }
      },
    }
  }

  assert!(part_buffer.is_empty());

  Ok((WordNode { parts }, src))
}

fn parse_curly_braced_string(mut src: &str) -> Result<(String, &str), ParseError> {
  match src.chars().next() {
    Some('{') => {
      src = &src[1..];
    }
    _ => return Err(ParseError::Generic("expected {".to_string())),
  }

  let mut depth = 1;
  let mut buffer = String::new();

  while !src.is_empty() {
    match src.chars().next().unwrap() {
      '{' => {
        buffer.push('{');
        depth += 1;
        src = &src[1..];
      }

      '}' => {
        depth -= 1;
        src = &src[1..];
        if depth == 0 {
          break;
        } else {
          buffer.push('}');
        }
      }

      '\\' => {
        if let Some(ch @ ('{' | '}')) = src.chars().nth(1) {
          buffer.push('\\');
          buffer.push(ch);
          src = &src[2..];
          continue;
        }

        if let Ok((ch, rest)) = parse_backslash_escape_newline(src) {
          buffer.push(ch);
          src = rest;
          continue;
        }

        buffer.push('\\');
        src = &src[1..];
      }

      ch => {
        buffer.push(ch);
        src = &src[ch.len_utf8()..];
      }
    }
  }

  if depth > 0 {
    return Err(ParseError::Continuable("missing closing }".to_string()));
  }

  Ok((buffer, src))
}

fn parse_backslash_escape(src: &str) -> Result<(char, &str), ParseError> {
  let Some('\\') = src.chars().next() else {
    return Err(ParseError::Generic("expected \\".to_string()));
  };
  let rest = &src[1..];

  if let Ok(result) = parse_backslash_escape_newline(src) {
    return Ok(result);
  }

  let from_radix = |string: String, radix: u32| {
    let parse_err =
      || ParseError::Generic(format!("failed to parse base-{} escape: {}", radix, string));
    let val = char::from_u32(u32::from_str_radix(string.as_str(), radix).map_err(|_| parse_err())?)
      .ok_or_else(|| parse_err())?;
    Ok(val)
  };

  static RE_OCTAL_ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-7]{1,3})").unwrap());
  if let Some(cap) = RE_OCTAL_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 8)?, rest));
  }

  static RE_HEX_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^x([0-9a-fA-F]{1,2})").unwrap());
  if let Some(cap) = RE_HEX_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 16)?, rest));
  }

  static RE_UNICODE_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^u([0-9a-fA-F]{1,6})").unwrap());
  if let Some(cap) = RE_UNICODE_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 16)?, rest));
  }

  static RE_SIMPLE_ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([abfnrtv])").unwrap());
  if let Some(cap) = RE_SIMPLE_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return match cap[1].to_string().as_str() {
      "a" => Ok(('\x07', rest)),
      "b" => Ok(('\x08', rest)),
      "f" => Ok(('\x0c', rest)),
      "n" => Ok(('\n', rest)),
      "r" => Ok(('\r', rest)),
      "t" => Ok(('\t', rest)),
      "v" => Ok(('\x0b', rest)),
      unsupported => Err(ParseError::Generic(format!(
        "unsupported character escape: {}",
        unsupported
      ))),
    };
  }

  // a backslash before any other character is that character
  if let Some(escaped) = rest.chars().next() {
    return Ok((escaped, &rest[escaped.len_utf8()..]));
  }

  // trailing backslash is a literal backslash
  Ok(('\\', rest))
}

fn parse_backslash_escape_newline(src: &str) -> Result<(char, &str), ParseError> {
  let Some('\\') = src.chars().next() else {
    return Err(ParseError::Generic("expected \\".to_string()));
  };
  let rest = &src[1..];

  static RE_NEWLINE_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((\r\n|\r|\n)[ \t]*)").unwrap());
  if let Some(cap) = RE_NEWLINE_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok(((' '), rest));
  }

  Err(ParseError::Generic("expected escaped newline".to_string()))
}

pub(crate) fn parse_ws_or_command_sep(src: &str) -> Result<(String, &str), ParseError> {
  static RE_WS_OR_CMD_SEP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\t\p{Zs}\r\n;]+").unwrap());

  if let Some(captures) = RE_WS_OR_CMD_SEP.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected whitespace".to_string()))
  }
}

fn parse_command_sep(src: &str) -> Result<(String, &str), ParseError> {
  static RE_CMD_SEP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\r\n;]+").unwrap());

  if let Some(captures) = RE_CMD_SEP.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic(
      "expected command separator (newline or `;`)".to_string(),
    ))
  }
}

pub(crate) fn parse_ws(src: &str) -> Result<(String, &str), ParseError> {
  static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\t\p{Zs}]+").unwrap());

  if let Some(captures) = RE_WS.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected whitespace".to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_char_escape_newline() -> Result<(), ParseError> {
    let (char, _rest) = parse_backslash_escape("\\n")?;
    assert_eq!(char, '\n');
    Ok(())
  }

  #[test]
  fn parses_word_with_two_varsubs() -> Result<(), ParseError> {
    let parsed = parse_word("$x$y")?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![
            WordPart::VarSub("x".to_string()),
            WordPart::VarSub("y".to_string())
          ]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_word_braced_sub() -> Result<(), ParseError> {
    let parsed = parse_word("${hello}")?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![WordPart::BracedSub("hello".to_string())]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_word_with_many_parts() -> Result<(), ParseError> {
    let parsed = parse_word("$x[expr 1 + 2]$y[a][b]{c}")?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![
            WordPart::VarSub("x".to_string()),
            WordPart::CommandSub("expr 1 + 2".to_string()),
            WordPart::VarSub("y".to_string()),
            WordPart::CommandSub("a".to_string()),
            WordPart::CommandSub("b".to_string()),
            WordPart::BareLiteral("{c}".to_string()),
          ]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_quoted_word_with_var_sub() -> Result<(), ParseError> {
    let parsed = parse_word(r#""hello $name""#)?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![WordPart::Quoted(vec![
            WordPart::BareLiteral("hello ".to_string()),
            WordPart::VarSub("name".to_string())
          ]),]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_quoted_word_with_command_sub() -> Result<(), ParseError> {
    let parsed = parse_word(r#""sum [expr 1 + 2]""#)?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![WordPart::Quoted(vec![
            WordPart::BareLiteral("sum ".to_string()),
            WordPart::CommandSub("expr 1 + 2".to_string())
          ]),]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_quoted_word_with_backslash_sub() -> Result<(), ParseError> {
    let parsed = parse_word(r#""a\nb\"c""#)?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![WordPart::Quoted(vec![WordPart::BareLiteral(
            "a\nb\"c".to_string()
          )])]
        },
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_backslash_escapes_in_bare_word() -> Result<(), ParseError> {
    let parsed = parse_word(r#"a\ b\;\$x\[cmd\]\{c\}\"q"#)?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral("a b;$x[cmd]{c}\"q".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_backslash_sequences_in_bare_word() -> Result<(), ParseError> {
    let parsed = parse_word(r"line\ncol\t\x41\101\q")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral("line\ncol\tAAq".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn trailing_backslash_in_bare_word_is_literal() -> Result<(), ParseError> {
    let parsed = parse_word(r"hello\")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral(r"hello\".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn braces_and_quotes_are_literal_inside_bare_word() -> Result<(), ParseError> {
    let parsed = parse_word(r#"pre{braced}"quoted""#)?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral(r#"pre{braced}"quoted""#.to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn braced_word_rejects_trailing_characters() {
    assert!(parse_word("{hello}world").is_err());
  }

  #[test]
  fn braced_word_allows_following_separator() -> Result<(), ParseError> {
    let parsed = parse_word("{hello} world")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral("hello".to_string())),
        " world"
      )
    );
    Ok(())
  }

  #[test]
  fn curly_braced_string_preserves_nesting_and_returns_remainder() -> Result<(), ParseError> {
    let parsed = parse_curly_braced_string("{a {nested} value}rest")?;
    assert_eq!(parsed, ("a {nested} value".to_string(), "rest"));
    Ok(())
  }

  #[test]
  fn curly_braced_string_reports_missing_close() {
    assert!(matches!(
      parse_curly_braced_string("{unclosed"),
      Err(ParseError::Continuable(_))
    ));
  }

  #[test]
  fn quoted_word_rejects_trailing_characters() {
    assert!(parse_word(r#""hello"world"#).is_err());
  }

  #[test]
  fn backslash_newline_continues_bare_word() -> Result<(), ParseError> {
    let parsed = parse_word("hello\\\n \t  world")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral("hello world".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn backslash_newline_continues_quoted_word() -> Result<(), ParseError> {
    let parsed = parse_word("\"hello\\\n \t  world\"")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::Quoted(vec![WordPart::BareLiteral(
          "hello world".to_string()
        )])),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn backslash_newline_is_substituted_in_braced_word() -> Result<(), ParseError> {
    let parsed = parse_word("{hello\\\n \t  world}")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral("hello world".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn braced_backslash_newline_does_not_consume_next_newline() -> Result<(), ParseError> {
    let parsed = parse_word("{hello\\\n\nworld}")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral("hello \nworld".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn braced_word_preserves_other_backslashes() -> Result<(), ParseError> {
    let parsed = parse_word(r"{a\n\$x}")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral(r"a\n\$x".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn escaped_brace_does_not_close_braced_word() -> Result<(), ParseError> {
    let parsed = parse_word(r"{a\}b}")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral(r"a\}b".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn escaped_open_brace_does_not_nest_braced_word() -> Result<(), ParseError> {
    let parsed = parse_word(r"{a\{b}")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BracedLiteral(r"a\{b".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn escaped_bracket_does_not_close_command_substitution() -> Result<(), ParseError> {
    let parsed = parse_word(r"[list \]]")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::CommandSub(r"list \]".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn escaped_open_bracket_does_not_nest_command_substitution() -> Result<(), ParseError> {
    let parsed = parse_word(r"[list \[]")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::CommandSub(r"list \[".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn command_substitution_ignores_close_bracket_in_braced_word() -> Result<(), ParseError> {
    let parsed = parse_word(r"[set x {]}]")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::CommandSub("set x {]}".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn lone_dollar_is_a_word() -> Result<(), ParseError> {
    let parsed = parse_word("$")?;
    assert_eq!(
      parsed,
      (WordNode::only(WordPart::BareLiteral("$".to_string())), "")
    );
    Ok(())
  }

  #[test]
  fn parses_word_includes_parentheses() -> Result<(), ParseError> {
    let parsed = parse_word("hello(a)")?;
    assert_eq!(
      parsed,
      (
        WordNode::only(WordPart::BareLiteral("hello(a)".to_string())),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_literal_newline() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\\n")?;
    assert_eq!(parsed, (' ', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_literal_newline_with_indent() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\\n  \t")?;
    assert_eq!(parsed, (' ', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_literal_newline_rn() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\\r\n  \t")?;
    assert_eq!(parsed, (' ', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_literal_newline_rest() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\\n  \trest")?;
    assert_eq!(parsed, (' ', "rest"));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_octal() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\041")?;
    assert_eq!(parsed, ('!', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_octal_short() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\41")?;
    assert_eq!(parsed, ('!', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_octal_invalid_as_literal() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\9")?;
    assert_eq!(parsed, ('9', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_hex() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\x41")?;
    assert_eq!(parsed, ('\x41', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_hex_short() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\xa")?;
    assert_eq!(parsed, ('\x0a', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_hex_invalid_as_literal() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\xq")?;
    assert_eq!(parsed, ('x', "q"));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_unicode() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\x10FFFF")?;
    assert_eq!(parsed, ('\u{10FFFF}', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_unicode_short() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\u2764")?;
    assert_eq!(parsed, ('\u{2764}', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_unicode_invalid_as_literal() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\uzzzz")?;
    assert_eq!(parsed, ('u', "zzzz"));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_simple() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\n")?;
    assert_eq!(parsed, ('\n', ""));
    Ok(())
  }

  #[test]
  fn parses_backslash_escape_unknown() -> Result<(), ParseError> {
    let parsed = parse_backslash_escape("\\z")?;
    assert_eq!(parsed, ('z', ""));
    Ok(())
  }

  #[test]
  fn parses_list_decodes_braced_element() -> Result<(), ParseError> {
    let parsed = parse_list("{args}")?;
    assert_eq!(parsed, (vec!["args".to_string()], ""));
    Ok(())
  }

  #[test]
  fn parses_list_leaves_substitution_syntax_literal() -> Result<(), ParseError> {
    let parsed = parse_list("$x [foo]")?;
    assert_eq!(parsed, (vec!["$x".to_string(), "[foo]".to_string()], ""));
    Ok(())
  }

  #[test]
  fn parses_script_with_one_set_command() -> Result<(), ParseError> {
    let parsed = parse("set x 3")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![CommandNode {
          words: vec![
            WordNode::only(WordPart::BareLiteral("set".to_string())),
            WordNode::only(WordPart::BareLiteral("x".to_string())),
            WordNode::only(WordPart::BareLiteral("3".to_string())),
          ]
        }]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_script_with_two_commands() -> Result<(), ParseError> {
    let parsed = parse("set x 3\nexpr 2 + 1")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![
          CommandNode {
            words: vec![
              WordNode::only(WordPart::BareLiteral("set".to_string())),
              WordNode::only(WordPart::BareLiteral("x".to_string())),
              WordNode::only(WordPart::BareLiteral("3".to_string())),
            ],
          },
          CommandNode {
            words: vec![
              WordNode::only(WordPart::BareLiteral("expr".to_string())),
              WordNode::only(WordPart::BareLiteral("2".to_string())),
              WordNode::only(WordPart::BareLiteral("+".to_string())),
              WordNode::only(WordPart::BareLiteral("1".to_string())),
            ]
          },
        ]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_script_with_var_sub() -> Result<(), ParseError> {
    let parsed = parse("expr 2 + $x")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![CommandNode {
          words: vec![
            WordNode::only(WordPart::BareLiteral("expr".to_string())),
            WordNode::only(WordPart::BareLiteral("2".to_string())),
            WordNode::only(WordPart::BareLiteral("+".to_string())),
            WordNode::only(WordPart::VarSub("x".to_string())),
          ]
        },]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_script_with_cmd_sub() -> Result<(), ParseError> {
    let parsed = parse("expr 2 + [expr 3 + [expr 4 + 5]]")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![CommandNode {
          words: vec![
            WordNode::only(WordPart::BareLiteral("expr".to_string())),
            WordNode::only(WordPart::BareLiteral("2".to_string())),
            WordNode::only(WordPart::BareLiteral("+".to_string())),
            WordNode::only(WordPart::CommandSub("expr 3 + [expr 4 + 5]".to_string())),
          ]
        },]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_script_with_quoted_and_braced_strings() -> Result<(), ParseError> {
    let parsed = parse(r#"puts "hello world" {nested {braced} string}"#)?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![CommandNode {
          words: vec![
            WordNode::only(WordPart::BareLiteral("puts".to_string())),
            WordNode::only(WordPart::Quoted(vec![WordPart::BareLiteral(
              "hello world".to_string()
            )])),
            WordNode::only(WordPart::BracedLiteral(
              "nested {braced} string".to_string()
            )),
          ]
        }]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_script_with_leading_cmd_sep() -> Result<(), ParseError> {
    let parsed = parse("\n;; puts hey;;;expr 1")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![
          CommandNode {
            words: vec![
              WordNode::only(WordPart::BareLiteral("puts".to_string())),
              WordNode::only(WordPart::BareLiteral("hey".to_string())),
            ]
          },
          CommandNode {
            words: vec![
              WordNode::only(WordPart::BareLiteral("expr".to_string())),
              WordNode::only(WordPart::BareLiteral("1".to_string())),
            ]
          },
        ]
      }
    );
    Ok(())
  }

  #[test]
  fn parses_cmd_with_leading_cmd_sep() -> Result<(), ParseError> {
    let (parsed, _) = parse_command("\n;; puts hey")?;
    assert_eq!(
      parsed,
      CommandNode {
        words: vec![
          WordNode::only(WordPart::BareLiteral("puts".to_string())),
          WordNode::only(WordPart::BareLiteral("hey".to_string())),
        ]
      },
    );
    Ok(())
  }
}
