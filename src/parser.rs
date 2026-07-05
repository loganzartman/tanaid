use regex::Regex;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

#[derive(Debug)]
pub enum ParseError {
  Generic(String),
  Internal(String),
  NotImplemented,
}

impl Display for ParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ParseError::Generic(s) => write!(f, "[ParseError] {}", s),
      ParseError::Internal(s) => write!(f, "[ParseError] internal error: {}", s),
      ParseError::NotImplemented => write!(f, "[ParseError] not implemented"),
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
  let (script_node, _) = parse_script(src)?;
  return Ok(script_node);
}

pub(crate) fn parse_script(mut src: &str) -> Result<(ScriptNode, &str), ParseError> {
  let mut commands: Vec<CommandNode> = vec![];

  while !src.is_empty() {
    if let Ok((command, rest)) = parse_command(src) {
      commands.push(command);
      src = rest;
    } else {
      return Err(ParseError::Generic("expected command".to_string()));
    }

    if let Ok((_, rest)) = parse_ws(src) {
      src = rest;
    }

    if let Ok((_, rest)) = parse_command_sep(src) {
      src = rest;
    } else {
      break;
    }

    // eat whitespace
    if let Ok((_, rest)) = parse_ws_or_command_sep(src) {
      src = rest;
    }
  }

  Ok((ScriptNode { commands }, src))
}

fn parse_command(mut src: &str) -> Result<(CommandNode, &str), ParseError> {
  // eat whitespace
  if let Ok((_, rest)) = parse_ws_or_command_sep(src) {
    src = rest;
  }

  // required: first word (command name)
  let (name, rest) = parse_word(src)
    .map_err(|e| ParseError::Generic(format!("expected command name\ncaused by: {}", e)))?;
  let mut words: Vec<WordNode> = vec![name];
  src = rest;

  // collect additional words
  while !src.is_empty() {
    // required whitespace separator
    let Ok((_, rest)) = parse_ws(src) else {
      break;
    };
    src = rest;

    // word
    let Ok((word, rest)) = parse_word(src) else {
      break;
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
    if let Ok((_, rest)) = parse_ws(src) {
      src = rest;
    } else if !first {
      break;
    }
    first = false;

    if let Ok((str, rest)) = parse_bracketed(src, BracketType::Curly) {
      items.push(str);
      src = rest;
    } else if let Ok((str, rest)) = parse_bracketed(src, BracketType::DoubleQuote) {
      items.push(str);
      src = rest;
    } else if let Ok((str, rest)) = parse_list_element_bare(src) {
      items.push(str);
      src = rest;
    } else {
      break;
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
  if let Ok((_, rest)) = parse_ws(src) {
    src = rest;
  }

  let mut parts: Vec<WordPart> = vec![];
  while !src.is_empty() {
    if let Ok((part, rest)) = parse_wordpart_quoted(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    if let Ok((part, rest)) = parse_wordpart_cmdsub(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    if let Ok((part, rest)) = parse_wordpart_varsub(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    if let Ok((part, rest)) = parse_wordpart_literal(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    break;
  }

  if parts.is_empty() {
    Err(ParseError::Generic("expected word".to_string()))
  } else {
    Ok((WordNode { parts }, src))
  }
}

fn parse_wordpart_quoted(src: &str) -> Result<(WordPart, &str), ParseError> {
  let Some(mut src) = src.strip_prefix("\"") else {
    return Err(ParseError::Generic("expected \"".to_string()));
  };

  let mut parts = vec![];
  while !src.is_empty() {
    if let Some(rest) = src.strip_prefix("\"") {
      return Ok((WordPart::Quoted(parts), rest));
    }

    if let Ok((part, rest)) = parse_wordpart_cmdsub(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    if let Ok((part, rest)) = parse_wordpart_varsub(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    if let Ok((part, rest)) = parse_wordpart_quoted_literal(src) {
      parts.push(part);
      src = rest;
      continue;
    }

    break;
  }

  Err(ParseError::Generic("expected \"".to_string()))
}

fn parse_wordpart_quoted_literal(src: &str) -> Result<(WordPart, &str), ParseError> {
  parse_wordpart_quoted_bare(src)
    .map(|(s, rest)| (WordPart::BareLiteral(s), rest))
    .map_err(|_| ParseError::Generic("expected literal word".to_string()))
}

fn parse_wordpart_literal(src: &str) -> Result<(WordPart, &str), ParseError> {
  if let Ok((s, rest)) = parse_bracketed(src, BracketType::Curly) {
    return Ok((WordPart::BracedLiteral(s), rest));
  }

  parse_wordpart_bare(src)
    .map(|(s, rest)| (WordPart::BareLiteral(s), rest))
    .map_err(|_| ParseError::Generic("expected literal word".to_string()))
}

fn parse_wordpart_varsub(src: &str) -> Result<(WordPart, &str), ParseError> {
  let rest = src
    .strip_prefix('$')
    .ok_or_else(|| ParseError::Generic("expected variable substitution".to_string()))?;

  if let Ok((word, rest)) = parse_bracketed(rest, BracketType::Curly) {
    return Ok((WordPart::BracedSub(word), rest));
  }

  parse_wordpart_bare(rest).map(|(word, rest)| (WordPart::VarSub(word), rest))
}

fn parse_wordpart_cmdsub(src: &str) -> Result<(WordPart, &str), ParseError> {
  let (word, rest) = parse_bracketed(src, BracketType::Square)?;
  Ok((WordPart::CommandSub(word), rest))
}

enum BracketType {
  Square,
  Curly,
  DoubleQuote,
}

fn parse_bracketed(src: &str, b: BracketType) -> Result<(String, &str), ParseError> {
  let open = match b {
    BracketType::Square => '[',
    BracketType::Curly => '{',
    BracketType::DoubleQuote => '"',
  };
  let close = match b {
    BracketType::Square => ']',
    BracketType::Curly => '}',
    BracketType::DoubleQuote => '"',
  };

  let mut rest = src
    .strip_prefix(open)
    .ok_or_else(|| ParseError::Generic(format!("expected a: {}", open)))?;

  let mut depth = 1;
  let mut word = String::new();

  while !rest.is_empty() {
    let (ch, new_rest) = parse_char(rest)?;
    rest = new_rest;

    if ch == close {
      depth -= 1;
      if depth == 0 {
        break;
      }
    }

    if ch == open {
      depth += 1;
    }

    word.push(ch);
  }

  if depth > 0 {
    Err(ParseError::Generic(format!("missing closing {}", close)))
  } else {
    Ok((word, rest))
  }
}

fn parse_wordpart_bare(src: &str) -> Result<(String, &str), ParseError> {
  static RE_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[^$\[\]{}()";\s]+"#).unwrap());

  if let Some(captures) = RE_WORD.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected bare word".to_string()))
  }
}

fn parse_wordpart_quoted_bare(src: &str) -> Result<(String, &str), ParseError> {
  let mut rest = src;
  let mut word = String::new();

  while let Some(ch) = rest.chars().next() {
    // Stop at substitution/terminator characters, but let parse_char handle
    // backslash escapes (so `\"` doesn't terminate the quoted string).
    if matches!(ch, '$' | '[' | ']' | '"') {
      break;
    }
    let (decoded, new_rest) = parse_char(rest)?;
    word.push(decoded);
    rest = new_rest;
  }

  if word.is_empty() {
    Err(ParseError::Generic("expected bare word".to_string()))
  } else {
    Ok((word, rest))
  }
}

fn parse_char(src: &str) -> Result<(char, &str), ParseError> {
  let ch = src
    .chars()
    .next()
    .ok_or_else(|| ParseError::Generic("expected character".to_string()))?;

  let rest = &src[ch.len_utf8()..];
  if ch != '\\' {
    return Ok((ch, rest));
  }

  static RE_OCTAL_ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-7]{1,3})").unwrap());
  static RE_HEX_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^x([0-9a-fA-F]{1,2})").unwrap());
  static RE_UNICODE_ESCAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^u([0-9a-fA-F]{1,6})").unwrap());
  static RE_SIMPLE_ESCAPE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([abfnrtv])").unwrap());

  let from_radix = |string: String, radix: u32| {
    let parse_err =
      || ParseError::Generic(format!("failed to parse base-{} escape: {}", radix, string));
    let val = char::from_u32(u32::from_str_radix(string.as_str(), radix).map_err(|_| parse_err())?)
      .ok_or_else(|| parse_err())?;
    Ok(val)
  };

  if let Some(cap) = RE_OCTAL_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 8)?, rest));
  }

  if let Some(cap) = RE_HEX_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 16)?, rest));
  }

  if let Some(cap) = RE_UNICODE_ESCAPE.captures(rest) {
    let rest = &rest[cap[0].len()..];
    return Ok((from_radix(cap[1].to_string(), 16)?, rest));
  }

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

  // Tcl semantics: a backslash before any other character is that character.
  if let Some(escaped) = rest.chars().next() {
    return Ok((escaped, &rest[escaped.len_utf8()..]));
  }

  // A lone trailing backslash is a literal backslash.
  Ok(('\\', rest))
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
  fn parse_char_ascii() -> Result<(), ParseError> {
    let (char, _rest) = parse_char("a")?;
    assert_eq!(char, 'a');
    Ok(())
  }

  #[test]
  fn parse_char_escape_newline() -> Result<(), ParseError> {
    let (char, _rest) = parse_char("\\n")?;
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
            WordPart::BracedLiteral("c".to_string()),
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
  fn parses_word_excludes_paren() -> Result<(), ParseError> {
    let parsed = parse_word("hello(a")?;
    assert_eq!(
      parsed,
      (
        WordNode {
          parts: vec![WordPart::BareLiteral("hello".to_string())]
        },
        "(a"
      )
    );
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
