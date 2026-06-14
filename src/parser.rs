use std::fmt;
use std::fmt::{Display, Formatter};

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

pub fn parse(src: &str) -> ScriptNode {
  return ScriptNode {
    commands: vec![CommandNode {
      words: vec![
        WordNode::Literal(String::from("expr")),
        WordNode::Literal(String::from("2")),
        WordNode::Literal(String::from("+")),
        WordNode::Literal(String::from("3")),
      ],
    }],
  };
}
