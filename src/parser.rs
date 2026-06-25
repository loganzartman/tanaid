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

#[derive(PartialEq, Debug)]
pub struct ScriptNode {
  pub commands: Vec<CommandNode>,
}

#[derive(PartialEq, Debug)]
pub struct CommandNode {
  pub words: Vec<WordNode>,
}

#[derive(PartialEq, Debug)]
pub struct WordNode {
  pub parts: Vec<WordPart>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum WordPart {
  BareLiteral(String),
  BracedLiteral(String),
  QuotedLiteral(String),
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
      QuotedLiteral(s) => write!(f, "\"{}\"", s),
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
    if let Ok((_, rest)) = parse_ws(src) {
      src = rest;
    }

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
  }

  Ok((ScriptNode { commands }, src))
}

fn parse_command(mut src: &str) -> Result<(CommandNode, &str), ParseError> {
  // eat whitespace
  if let Ok((_, rest)) = parse_ws(src) {
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

fn parse_command_sep(src: &str) -> Result<(String, &str), ParseError> {
  static RE_CMD_SEP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\r\n\;]+").unwrap());

  if let Some(captures) = RE_CMD_SEP.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic(
      "expected command separator (newline or `;`)".to_string(),
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

fn parse_wordpart_literal(src: &str) -> Result<(WordPart, &str), ParseError> {
  if let Ok((s, rest)) = parse_bracketed(src, BracketType::Curly) {
    return Ok((WordPart::BracedLiteral(s), rest));
  }

  if let Ok((s, rest)) = parse_bracketed(src, BracketType::DoubleQuote) {
    return Ok((WordPart::QuotedLiteral(s), rest));
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

fn parse_char(src: &str) -> Result<(char, &str), ParseError> {
  let ch = src
    .chars()
    .next()
    .ok_or_else(|| ParseError::Generic("expected character".to_string()))?;

  if ch != '\\' {
    return Ok((ch, &src[1..]));
  }

  let next = src
    .chars()
    .next()
    .ok_or_else(|| ParseError::Generic("expected escape sequence".to_string()))?;

  match next {
    'n' => Ok(('\n', &src[2..])),
    't' => Ok(('\t', &src[2..])),
    'r' => Ok(('\r', &src[2..])),
    _ => Ok((next, &src[2..])),
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
            WordNode::only(WordPart::QuotedLiteral("hello world".to_string())),
            WordNode::only(WordPart::BracedLiteral(
              "nested {braced} string".to_string()
            )),
          ]
        }]
      }
    );
    Ok(())
  }
}
