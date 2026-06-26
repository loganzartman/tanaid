use crate::eval_error::EvalError;
use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use crate::parser_expr;
use crate::parser_expr::{BinaryOp, ExprNode};
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

pub fn eval(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  eval_script(script, context)
}

pub fn eval_script(script: &ScriptNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    result = eval_command(&command, context)?;
  }
  Ok(result)
}

pub fn eval_command(command: &CommandNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  let [name, args @ ..] = command.words.as_slice() else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let mut name_value = eval_word(&name, context)?;

  match name_value.repr_str()? {
    "break" => eval_cmd_break(args, context),
    "expr" => eval_cmd_expr(args, context),
    "if" => eval_cmd_if(args, context),
    "puts" => eval_cmd_puts(args, context),
    "set" => eval_cmd_set(args, context),
    "while" => eval_cmd_while(args, context),
    _ => Err(EvalError::NotImplemented),
  }
}

pub fn eval_cmd_break(_words: &[WordNode], _context: &mut EvalContext) -> Result<Value, EvalError> {
  Err(EvalError::BreakError)
}

pub fn eval_cmd_expr(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let values = words
    .iter()
    .map(|word| eval_word(&word, context).map(|value| value.to_string()));
  let joined = values.collect::<Result<Vec<String>, _>>()?.join(" ");
  let (node, _) = parser_expr::parse_expr(joined.as_str())
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  eval_expr(&node, context)
}

pub fn eval_cmd_if(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let mut args = words.iter().map(|w| eval_word(w, context)).peekable();

  let cond = &args
    .next()
    .ok_or_else(|| EvalError::ArgumentError("expected condition".to_string()))??;

  todo!()
}

pub fn eval_cmd_puts(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let [mut string] = match words {
    [_, _, _] => todo!(),
    [_, _] => todo!(),
    [string] => [eval_word(string, context)?],
    [..] => {
      return Err(EvalError::Generic(
        "too many arguments; expects string".to_string(),
      ));
    }
  };

  println!("{}", string.repr_str()?);
  Ok(Value::none())
}

pub fn eval_cmd_set(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let [name, value] = match words {
    [name, value] => [name, value],
    [_] => return Err(EvalError::Generic("missing value".to_string())),
    [] => return Err(EvalError::Generic("missing name and value".to_string())),
    _ => {
      return Err(EvalError::Generic(
        "too many arguments; expects name and value".to_string(),
      ));
    }
  };

  let mut name = eval_word(&name, context)?;
  let value = eval_word(&value, context)?;
  context.set_variable(name.repr_str()?, value.clone());
  Ok(value)
}

pub fn eval_cmd_while(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let [test, body] = words else {
    return Err(EvalError::Generic(
      "while requires two arguments: test and body".to_string(),
    ));
  };

  let (test_expr, _) = parser_expr::parse_expr(eval_word(test, context)?.repr_str()?)
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;

  let (body_script, _) = parser::parse_script(eval_word(body, context)?.repr_str()?)
    .map_err(|e| EvalError::ScriptParseError(e.to_string()))?;

  while eval_expr(&test_expr, context)?.repr_int()? != 0 {
    match eval_script(&body_script, context) {
      Err(EvalError::BreakError) => break,
      Err(e) => return Err(e),
      Ok(_) => {}
    }
  }

  Ok(Value::none())
}

pub fn eval_expr(node: &ExprNode, context: &mut EvalContext) -> Result<Value, EvalError> {
  use ExprNode::*;
  match node {
    Word(w) => eval_word(w, context),
    UnaryOp(_o, _x) => todo!(),
    BinaryOp(o, a, b) => eval_expr_binary_op(o, a.as_ref(), b.as_ref(), context),
    Ternary(_c, _i, _e) => todo!(),
  }
}

pub fn eval_expr_binary_op(
  o: &BinaryOp,
  a: &ExprNode,
  b: &ExprNode,
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  use BinaryOp::*;
  let mut a = eval_expr(a, context)?;
  let mut b = eval_expr(b, context)?;
  match o {
    Lt => a.lt(&mut b),
    Le => a.le(&mut b),
    Eq => a.eq(&mut b),
    Ne => a.ne(&mut b),
    Ge => a.ge(&mut b),
    Gt => a.gt(&mut b),
    Add => a + b,
    Sub => a - b,
    Mul => a * b,
    Div => a / b,
  }
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
      .and_then(|script| eval(&script, context)),
    WordPart::QuotedLiteral(s) => Ok(Value::new(s)),
    WordPart::VarSub(v) => context
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::VarIndex(_, _) => Err(EvalError::NotImplemented),
    WordPart::BracedIndex(_, _) => Err(EvalError::NotImplemented),
  }
}
