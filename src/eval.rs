use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use crate::parser_expr::ExprNode;
use crate::parser_expr::{self, BinaryOp};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops;

#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  UndefinedVariable(String),
  CommandParseError(String),
  ExprParseError(String),
  NotImplemented,
}

impl Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use EvalError::*;
    match self {
      Generic(s) => write!(f, "{}", s),
      UndefinedVariable(v) => write!(f, "Undefined variable: {}", v),
      CommandParseError(e) => write!(f, "Failed to parse command: {}", e),
      ExprParseError(e) => write!(f, "Failed to parse expr: {}", e),
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
  Float(f64),
}

#[derive(Clone, Debug)]
pub struct Value {
  string: String,
  repr: Repr,
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

  pub fn to_string(&mut self) -> Result<&String, EvalError> {
    Ok(&self.string)
  }

  pub fn to_int(&mut self) -> Result<i64, EvalError> {
    if let Repr::Int(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .string
      .parse::<i64>()
      .map_err(|e| EvalError::Generic(e.to_string()))?;
    self.repr = Repr::Int(x);
    Ok(x)
  }

  pub fn to_float(&mut self) -> Result<f64, EvalError> {
    if let Repr::Float(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .string
      .parse::<f64>()
      .map_err(|e| EvalError::Generic(e.to_string()))?;
    self.repr = Repr::Float(x);
    Ok(x)
  }
}

trait ToValue {
  fn to_value(&self) -> Value;
}

impl ToValue for String {
  fn to_value(&self) -> Value {
    Value {
      string: self.to_string(),
      repr: Repr::None,
    }
  }
}

impl ToValue for i64 {
  fn to_value(&self) -> Value {
    Value {
      string: self.to_string(),
      repr: Repr::Int(*self),
    }
  }
}

impl ToValue for f64 {
  fn to_value(&self) -> Value {
    Value {
      string: self.to_string(),
      repr: Repr::Float(*self),
    }
  }
}

impl ops::Add for Value {
  type Output = Result<Value, EvalError>;

  fn add(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.to_int(), rhs.to_int()) {
      return Ok((a + b).to_value());
    }
    Ok((self.to_float()? + rhs.to_float()?).to_value())
  }
}

impl ops::Sub for Value {
  type Output = Result<Value, EvalError>;

  fn sub(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.to_int(), rhs.to_int()) {
      return Ok((a - b).to_value());
    }
    Ok((self.to_float()? - rhs.to_float()?).to_value())
  }
}

impl ops::Mul for Value {
  type Output = Result<Value, EvalError>;

  fn mul(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.to_int(), rhs.to_int()) {
      return Ok((a * b).to_value());
    }
    Ok((self.to_float()? * rhs.to_float()?).to_value())
  }
}

impl ops::Div for Value {
  type Output = Result<Value, EvalError>;

  fn div(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.to_int(), rhs.to_int()) {
      return Ok((a / b).to_value());
    }
    Ok((self.to_float()? / rhs.to_float()?).to_value())
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

  let name_value = eval_word(&name, context)?;

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
  let values = words.map(|word| eval_word(&word, context).map(|word| word.string));
  let joined = values.collect::<Result<Vec<String>, _>>()?.join(" ");
  let (node, _) = parser_expr::parse_expr(joined.as_str())
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  eval_expr_node(&node, context)
}

pub fn eval_expr_node(node: &ExprNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  use ExprNode::*;
  match node {
    Word(w) => eval_word(w, context),
    UnaryOp(o, x) => todo!(),
    BinaryOp(o, a, b) => eval_expr_binary_op(o, a.as_ref(), b.as_ref(), context),
    Ternary(c, i, e) => todo!(),
  }
}

pub fn eval_expr_binary_op(
  o: &BinaryOp,
  a: &ExprNode,
  b: &ExprNode,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  use BinaryOp::*;
  let a = eval_expr_node(a, context)?;
  let b = eval_expr_node(b, context)?;
  match o {
    Add => a + b,
    Sub => a - b,
    Mul => a * b,
    Div => a / b,
  }
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

  let name = eval_word(&name, context)?;
  let value = eval_word(&value, context)?;
  context.set_variable(name.string, value.clone());
  Ok(value)
}

pub fn eval_word(word: &WordNode, context: &mut EvalContext) -> Result<Value, EvalError> {
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
