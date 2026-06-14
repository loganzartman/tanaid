use std::fmt;
use std::fmt::{Display, Formatter};

use clap::Command;

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

pub enum AstNode {
  ScriptNode,
  CommandNode,
  WordNode,
}

pub struct ScriptNode {
  pub commands: Vec<CommandNode>,
}

pub struct CommandNode {
  pub words: Vec<WordNode>,
}

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
  return parse_script(src);
}

pub fn parse_script(mut src: &str) -> Result<ScriptNode, ParseError> {
  let mut commands: Vec<CommandNode> = vec![];

  while !src.is_empty() {
    if let Ok((_, new_src)) = parse_ws(src) {
      src = new_src;
    }
    if let Ok((command, new_src)) = parse_command(src) {
      commands.push(command);
      src = new_src;
    }
  }

  Ok(ScriptNode { commands })
}

pub fn parse_command(_src: &str) -> Result<(CommandNode, &str), ParseError> {
  return Err(ParseError::NotImplemented);
}

pub fn parse_word(_src: &str) -> Result<WordNode, ParseError> {
  return Err(ParseError::NotImplemented);
}

pub fn parse_ws(mut src: &str) -> Result<(String, &str), ParseError> {
  let mut result = String::new();

  while let Some(ch @ (' ' | '\t' | '\r' | '\n')) = src.chars().next() {
    result.push(ch);
    src = &src[ch.len_utf8()..];
  }

  if result.is_empty() {
    Err(ParseError::Generic("expected whitespace".to_string()))
  } else {
    Ok((result, src))
  }
}
