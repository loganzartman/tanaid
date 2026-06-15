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

#[derive(Debug)]
pub struct ScriptNode {
  pub commands: Vec<CommandNode>,
}

#[derive(Debug)]
pub struct CommandNode {
  pub words: Vec<WordNode>,
}

#[derive(Debug)]
pub enum WordNode {
  Literal(String),
  VarSub(String),
  CommandSub(ScriptNode),
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
    if let Ok((_, new_src)) = parse_ws(src) {
      src = new_src;
    }

    if let Ok((command, new_src)) = parse_command(src) {
      commands.push(command);
      src = new_src;
    } else {
      return Err(ParseError::Generic("expected command".to_string()));
    }

    if let Ok((_, new_src)) = parse_ws(src) {
      src = new_src;
    }

    if let Err(_) = parse_command_sep(src) {
      break;
    }
  }

  Ok((ScriptNode { commands }, src))
}

fn parse_command(mut src: &str) -> Result<(CommandNode, &str), ParseError> {
  if let Ok((_, new_src)) = parse_ws(src) {
    src = new_src;
  }

  let (name, new_src) = parse_word(src).map_err(|e| {
    ParseError::Generic(format!("expected command name\ncaused by: {}", e).to_string())
  })?;
  let mut words: Vec<WordNode> = vec![name];
  src = new_src;

  while !src.is_empty() {
    if let Ok((_, new_src)) = parse_ws(src) {
      src = new_src;
    } else {
      break;
    }

    if let Ok((word, new_src)) = parse_word(src) {
      words.push(word);
      src = new_src;
    } else {
      break;
    }
  }

  Ok((CommandNode { words }, src))
}

fn parse_command_sep(_src: &str) -> Result<(String, &str), ParseError> {
  Err(ParseError::NotImplemented)
}

fn parse_word(src: &str) -> Result<(WordNode, &str), ParseError> {
  if let Ok(result) = parse_word_literal(src) {
    return Ok(result);
  }

  Err(ParseError::Generic("expected word".to_string()))
}

fn parse_word_literal(src: &str) -> Result<(WordNode, &str), ParseError> {
  let re_word = Regex::new(r#"^[^\[\]{}";\s]+"#).unwrap();

  if let Some(captures) = re_word.captures(src) {
    Ok((
      WordNode::Literal(captures[0].to_string()),
      &src[captures[0].len()..],
    ))
  } else {
    Err(ParseError::Generic("expected literal word".to_string()))
  }
}

fn parse_ws(src: &str) -> Result<(String, &str), ParseError> {
  let re_ws = Regex::new(r"^\h+").unwrap();

  if let Some(captures) = re_ws.captures(src) {
    Ok((captures[0].to_string(), &src[captures[0].len()..]))
  } else {
    Err(ParseError::Generic("expected whitespace".to_string()))
  }
}
