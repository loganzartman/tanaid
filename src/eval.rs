use crate::parser::{CommandNode, ScriptNode, WordNode};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  NotImplemented,
}

impl Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self)
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
  mut words: impl Iterator<Item = WordNode>,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  eval_word(
    words
      .next()
      .ok_or(EvalError::Generic("missing expression".to_string()))?,
    context,
  )
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
  match word {
    WordNode::Literal(s) => Ok(Value::new(s)),
    WordNode::VarSub(name) => context
      .get_variable(name.as_str())
      .cloned()
      .ok_or(EvalError::Generic(format!("variable {} not found", name))),
    _ => Err(EvalError::NotImplemented),
  }
}
