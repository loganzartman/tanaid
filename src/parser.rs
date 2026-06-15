use regex::Regex;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ParseError {
  Generic(String),
  NotImplemented,
}

impl Display for ParseError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ParseError::Generic(s) => write!(f, "[ParseError] {}", s),
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
pub enum WordNode {
  Literal(String),
  VarSub(String),
  CommandSub(String),
  Concat(Vec<WordNode>),
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
    use WordNode::*;
    match self {
      Literal(s) => write!(f, "{}", s),
      VarSub(s) => write!(f, "${}", s),
      CommandSub(n) => write!(f, "[{}]", n.to_string()),
      Concat(v) => {
        for n in v.iter() {
          write!(f, "{}", n.to_string())?;
        }
        Ok(())
      }
    }
  }
}

pub fn parse(src: &str) -> Result<ScriptNode, ParseError> {
  let (script_node, _) = parse_script(src)?;
  return Ok(script_node);
}

fn parse_script(mut src: &str) -> Result<(ScriptNode, &str), ParseError> {
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
  let re_cmd_sep = Regex::new(r"^[\r\n\;]+").unwrap();

  if let Some(captures) = re_cmd_sep.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic(
      "expected command separator (newline or `;`)".to_string(),
    ))
  }
}

fn parse_word(src: &str) -> Result<(WordNode, &str), ParseError> {
  if let Ok(result) = parse_word_cmdsub(src) {
    return Ok(result);
  }

  if let Ok(result) = parse_word_varsub(src) {
    return Ok(result);
  }

  if let Ok(result) = parse_word_literal(src) {
    return Ok(result);
  }

  Err(ParseError::Generic("expected word".to_string()))
}

fn parse_word_literal(src: &str) -> Result<(WordNode, &str), ParseError> {
  if let Ok((word, rest)) = parse_word_bracketed(src, BracketType::Curly) {
    return Ok((WordNode::Literal(word), rest));
  }

  if let Ok((word, rest)) = parse_word_bracketed(src, BracketType::DoubleQuote) {
    return Ok((WordNode::Literal(word), rest));
  }

  parse_word_bare(src).map(|(word, rest)| (WordNode::Literal(word), rest))
}

fn parse_word_varsub(src: &str) -> Result<(WordNode, &str), ParseError> {
  let rest = src
    .strip_prefix('$')
    .ok_or_else(|| ParseError::Generic("expected variable substitution".to_string()))?;
  parse_word_bare(rest).map(|(word, rest)| (WordNode::VarSub(word), rest))
}

fn parse_word_cmdsub(src: &str) -> Result<(WordNode, &str), ParseError> {
  let (word, rest) = parse_word_bracketed(src, BracketType::Square)?;
  Ok((WordNode::CommandSub(word), rest))
}

enum BracketType {
  Square,
  Curly,
  DoubleQuote,
}

fn parse_word_bracketed(src: &str, b: BracketType) -> Result<(String, &str), ParseError> {
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

fn parse_word_bare(src: &str) -> Result<(String, &str), ParseError> {
  let re_word = Regex::new(r#"^[^\[\]{}";\s]+"#).unwrap();

  if let Some(captures) = re_word.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected literal word".to_string()))
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

fn parse_ws(src: &str) -> Result<(String, &str), ParseError> {
  let re_ws = Regex::new(r"^[\t\p{Zs}]+").unwrap();

  if let Some(captures) = re_ws.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected whitespace".to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_script_with_one_set_command() -> Result<(), ParseError> {
    let parsed = parse("set x 3")?;
    assert_eq!(
      parsed,
      ScriptNode {
        commands: vec![CommandNode {
          words: vec![
            WordNode::Literal("set".to_string()),
            WordNode::Literal("x".to_string()),
            WordNode::Literal("3".to_string()),
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
              WordNode::Literal("set".to_string()),
              WordNode::Literal("x".to_string()),
              WordNode::Literal("3".to_string()),
            ],
          },
          CommandNode {
            words: vec![
              WordNode::Literal("expr".to_string()),
              WordNode::Literal("2".to_string()),
              WordNode::Literal("+".to_string()),
              WordNode::Literal("1".to_string()),
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
            WordNode::Literal("expr".to_string()),
            WordNode::Literal("2".to_string()),
            WordNode::Literal("+".to_string()),
            WordNode::VarSub("x".to_string()),
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
            WordNode::Literal("expr".to_string()),
            WordNode::Literal("2".to_string()),
            WordNode::Literal("+".to_string()),
            WordNode::CommandSub("expr 3 + [expr 4 + 5]".to_string()),
          ]
        },]
      }
    );
    Ok(())
  }
}
