use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  UndefinedVariable(String),
  CommandParseError(String),
  NotImplemented,
}

impl Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use EvalError::*;
    match self {
      Generic(s) => write!(f, "{}", s),
      UndefinedVariable(v) => write!(f, "Undefined variable: {}", v),
      CommandParseError(e) => write!(f, "Failed to parse command: {}", e),
      NotImplemented => write!(f, "Not implemented"),
    }
  }
}

impl std::error::Error for EvalError {}

pub struct EvalContext {
  variables: HashMap<String, Value>,
}

impl EvalContext {
  pub fn new() -> EvalContext {
    EvalContext {
      variables: HashMap::new(),
    }
  }

  pub fn get_variable(&self, name: &str) -> Option<&Value> {
    self.variables.get(name)
  }

  pub fn set_variable(&mut self, name: String, value: Value) {
    self.variables.insert(name, value);
  }
}

#[derive(Clone, Debug)]
pub enum Repr {
  None,
  Int(i64),
  Number(f64),
}

#[derive(Clone, Debug)]
pub struct Value {
  pub string: String,
  pub repr: Repr,
}

impl Value {
  pub fn none() -> Value {
    Value {
      string: "".to_string(),
      repr: Repr::None,
    }
  }

  pub fn new(string: String) -> Value {
    Value {
      string,
      repr: Repr::None,
    }
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.string)
  }
}

pub fn eval(script: ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  eval_script(script, context)
}

pub fn eval_script(script: ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in script.commands {
    result = eval_command(command, context)?;
  }
  Ok(result)
}

pub fn eval_command(command: CommandNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut words = command.words.into_iter();
  let Some(name) = words.next() else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let name_value = eval_word(name, context)?;

  match name_value.string.as_str() {
    "expr" => eval_expr(words, context),
    "set" => eval_set(words, context),
    _ => Err(EvalError::NotImplemented),
  }
}

pub fn eval_expr(
  words: impl Iterator<Item = WordNode>,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  let values = words.map(|word| eval_word(word, context).map(|word| word.string));
  let joined = values.collect::<Result<Vec<String>, _>>()?.join(" ");
  Ok(Value::new(joined))
}

pub fn eval_set(
  mut words: impl Iterator<Item = WordNode>,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  let Some(name) = words.next() else {
    return Err(EvalError::Generic("missing variable name".to_string()));
  };
  let Some(value) = words.next() else {
    return Err(EvalError::Generic("missing variable value".to_string()));
  };

  let name = eval_word(name, context)?;
  let value = eval_word(value, context)?;
  context.set_variable(name.string, value.clone());
  Ok(value)
}

pub fn eval_word(word: WordNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let joined = word
    .parts
    .iter()
    .map(|part| eval_wordpart(part.clone(), context).map(|p| p.string))
    .collect::<Result<Vec<String>, _>>()?
    .join("");
  Ok(Value::new(joined))
}

pub fn eval_wordpart(part: WordPart, context: &mut EvalContext) -> Result<Value, EvalError> {
  match part {
    WordPart::BareLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedSub(v) => context
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v))
      .cloned(),
    WordPart::CommandSub(c) => parser::parse(&c)
      .map_err(|e| EvalError::CommandParseError(e.to_string()))
      .and_then(|script| eval(script, context)),
    WordPart::QuotedLiteral(s) => Ok(Value::new(s)),
    WordPart::VarSub(v) => context
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v))
      .cloned(),
    WordPart::VarIndex(_, _) => Err(EvalError::NotImplemented),
    WordPart::BracedIndex(_, _) => Err(EvalError::NotImplemented),
  }
}
