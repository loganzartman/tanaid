use crate::eval_error::EvalError;
use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use crate::parser_expr::ExprNode;
use crate::parser_expr::{self, BinaryOp};
use crate::value::Value;
use std::collections::HashMap;

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

  pub fn set_variable(&mut self, name: &str, value: Value) {
    self.variables.insert(name.to_string(), value);
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

  let mut name_value = eval_word(&name, context)?;

  match name_value.repr_str()? {
    "expr" => eval_expr(words, context),
    "set" => eval_set(words, context),
    _ => Err(EvalError::NotImplemented),
  }
}

pub fn eval_expr(
  words: impl Iterator<Item = WordNode>,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  let values = words.map(|word| eval_word(&word, context).map(|value| value.to_string()));
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

  let mut name = eval_word(&name, context)?;
  let value = eval_word(&value, context)?;
  context.set_variable(name.repr_str()?, value.clone());
  Ok(value)
}

pub fn eval_word(word: &WordNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut joined = String::new();
  for part in &word.parts {
    let mut value = eval_wordpart(part, context)?;
    joined.push_str(value.repr_str()?);
  }
  Ok(Value::new(joined))
}

pub fn eval_wordpart(part: &WordPart, context: &mut EvalContext) -> Result<Value, EvalError> {
  match part {
    WordPart::BareLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedSub(v) => context
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::CommandSub(c) => parser::parse(&c)
      .map_err(|e| EvalError::CommandParseError(e.to_string()))
      .and_then(|script| eval(script, context)),
    WordPart::QuotedLiteral(s) => Ok(Value::new(s)),
    WordPart::VarSub(v) => context
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::VarIndex(_, _) => Err(EvalError::NotImplemented),
    WordPart::BracedIndex(_, _) => Err(EvalError::NotImplemented),
  }
}
