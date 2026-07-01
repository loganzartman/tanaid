use crate::eval_error::EvalError;
use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use crate::parser_expr;
use crate::parser_expr::{BinaryOp, ExprNode};
use crate::value::Value;
use std::collections::HashMap;

#[derive(PartialEq, Clone, Debug)]
pub struct Proc {
  params: Vec<String>,
  body: ScriptNode,
}

#[derive(Clone, Debug)]
pub struct EvalContext {
  variables: HashMap<String, Value>,
  procs: HashMap<String, Proc>,
}

impl EvalContext {
  pub fn new() -> EvalContext {
    EvalContext {
      variables: HashMap::new(),
      procs: HashMap::new(),
    }
  }

  pub fn fork(&self) -> EvalContext {
    // TODO: parent chain
    self.clone()
  }

  pub fn get_variable(&self, name: &str) -> Option<&Value> {
    self.variables.get(name)
  }

  pub fn set_variable(&mut self, name: &str, value: Value) {
    self.variables.insert(name.to_string(), value);
  }

  pub fn get_proc(&self, name: &str) -> Option<&Proc> {
    self.procs.get(name)
  }

  pub fn set_proc(&mut self, name: &str, proc: Proc) {
    self.procs.insert(name.to_string(), proc);
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
  let name_str = name_value.repr_str()?;

  // user-defined proc
  // TODO: reference-count procs
  if let Some(proc) = context.get_proc(name_str).cloned() {
    return eval_proc(name_str, &proc, args, context);
  }

  // builtin
  match name_str {
    "break" => eval_cmd_break(args, context),
    "expr" => eval_cmd_expr(args, context),
    "if" => eval_cmd_if(args, context),
    "proc" => eval_cmd_proc(args, context),
    "puts" => eval_cmd_puts(args, context),
    "set" => eval_cmd_set(args, context),
    "while" => eval_cmd_while(args, context),
    _ => Err(EvalError::UndefinedCommand(name_str.to_string())),
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

  let mut cond_body: Vec<(Value, Value)> = vec![];

  loop {
    // require `elseif` to start 2nd condition onward
    if !cond_body.is_empty() {
      if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "elseif") {
        args.next();
      } else {
        break;
      }
    }

    // required condition
    let cond = args
      .next()
      .ok_or_else(|| EvalError::ArgumentError("expected condition".to_string()))??;

    // optional "then"
    if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "then") {
      args.next();
    }

    // required body
    let body = args
      .next()
      .ok_or_else(|| EvalError::ArgumentError("expected condition body".to_string()))??;

    cond_body.push((cond, body));
  }

  // optional "else"
  if matches!(args.peek(), Some(Ok(value)) if value.to_string() == "else") {
    args.next();
  }

  // optional else body
  if let Some(else_body) = args.next() {
    cond_body.push((Value::from(1), else_body?));
  }

  for (cond, body) in &mut cond_body {
    let (cond_parsed, _) = parser_expr::parse_expr(cond.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if condition: {}", e)))?;
    let body_parsed = parser::parse(body.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if body: {}", e)))?;

    if eval_expr(&cond_parsed, context)?.repr_int()? != 0 {
      return eval(&body_parsed, context);
    }
  }

  Ok(Value::none())
}

pub fn eval_cmd_proc(words: &[WordNode], context: &mut EvalContext) -> Result<Value, EvalError> {
  let (mut name_val, mut params_val, mut body_val) = match words {
    [name, params, body] => (
      eval_word(name, context)?,
      eval_word(params, context)?,
      eval_word(body, context)?,
    ),
    [..] => {
      return Err(EvalError::ArgumentError(
        "wrong number of arguments; expects: proc name params body".to_string(),
      ));
    }
  };

  let name = name_val.repr_str()?;

  // args list items are not eval'ed; parse list and convert to literal strings
  let (params, "") = parser::parse_list(params_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc params must be a list: {}", e)))?
  else {
    return Err(EvalError::ArgumentError(
      "proc params must be a list: trailing input".to_string(),
    ));
  };

  let (body, "") = parser::parse_script(body_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc body must be a script: {}", e)))?
  else {
    return Err(EvalError::ArgumentError(
      "proc body must be a script: trailing input".to_string(),
    ));
  };

  context.set_proc(name, Proc { params, body });
  Ok(Value::none())
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

pub fn eval_proc(
  name: &str,
  proc: &Proc,
  args: &[WordNode],
  context: &mut EvalContext,
) -> Result<Value, EvalError> {
  let mut proc_context = context.fork();

  // bind arguments
  let mut args_it = args.iter().map(|arg| eval_word(arg, context));
  for (i, param) in proc.params.iter().enumerate() {
    // handle rest args
    if i == proc.params.len() - 1 {
      if param == "args" {
        let args_concat = args_it
          .by_ref()
          .map(|arg| arg?.repr_str().map(|str| str.to_string()))
          .collect::<Result<Vec<_>, _>>()?
          .join(" ");
        proc_context.set_variable("args", Value::new(args_concat));
        break;
      }
    }

    proc_context.set_variable(
      param,
      args_it
        .next()
        .ok_or_else(|| EvalError::ArgumentError(format!("not enough args for {}", name)))??,
    );
  }

  if args_it.next().is_some() {
    return Err(EvalError::ArgumentError(format!(
      "too many args for {}",
      name
    )));
  }

  eval_script(&proc.body, &mut proc_context)
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

#[cfg(test)]
mod tests {
  use crate::eval::*;
  use std::assert_matches;

  #[test]
  fn eval_if_simple() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("if {$x} {expr yes} {expr no}")?;
    {
      let mut ctx = EvalContext::new();
      ctx.set_variable("x", 1.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "yes");
    }
    {
      let mut ctx = EvalContext::new();
      ctx.set_variable("x", 0.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "no");
    }
    Ok(())
  }

  #[test]
  fn eval_if_verbose() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("if {$x} then {expr yes} else {expr no}")?;
    {
      let mut ctx = EvalContext::new();
      ctx.set_variable("x", 1.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "yes");
    }
    {
      let mut ctx = EvalContext::new();
      ctx.set_variable("x", 0.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "no");
    }
    Ok(())
  }

  #[test]
  fn eval_if_elseif_one_verbose() -> Result<(), Box<dyn std::error::Error>> {
    {
      let ast = parser::parse("if {0} then {expr 0} elseif {1} then {expr 1} else {expr 2}")?;
      let mut ctx = EvalContext::new();
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "1");
    }
    {
      let ast = parser::parse("if {0} then {expr 0} elseif {0} then {expr 1} else {expr 2}")?;
      let mut ctx = EvalContext::new();
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "2");
    }
    Ok(())
  }

  #[test]
  fn eval_if_elseif_three_verbose() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse(
      "if {0} then {expr 0} elseif {0} then {expr 1} elseif {1} then {expr 2} else {expr 3}",
    )?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "2");
    Ok(())
  }

  macro_rules! lit {
    ($val: expr) => {
      WordNode {
        parts: vec![WordPart::BareLiteral($val.to_string())],
      }
    };
  }

  #[test]
  fn eval_proc_no_args() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc hi {} {expr hey}")?;
    let mut ctx = EvalContext::new();
    eval(&ast, &mut ctx)?;
    assert_eq!(
      ctx.get_proc("hi"),
      Some(&Proc {
        params: vec![],
        body: ScriptNode {
          commands: vec![CommandNode {
            words: vec![lit!("expr"), lit!("hey")]
          }]
        }
      })
    );
    Ok(())
  }

  #[test]
  fn eval_proc_args_invoke() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2 3")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_int()?, 6);
    Ok(())
  }

  #[test]
  fn eval_proc_args_rest_invoke() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
      parser::parse("proc drop_first_two {x y args} {expr \"$args\"}; drop_first_two a b c d e")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "c d e");
    Ok(())
  }

  #[test]
  fn eval_proc_braced_args_param_is_rest_arg() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc collect {{args}} {expr \"$args\"}; collect a b c")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "a b c");
    Ok(())
  }

  #[test]
  fn eval_proc_braced_dollar_param_is_literal() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("set x args; proc f {$x} {expr \"$args\"}; f a b")?;
    let mut ctx = EvalContext::new();
    let result = eval(&ast, &mut ctx);
    assert_matches!(result, Err(EvalError::ArgumentError(_)));
    Ok(())
  }

  #[test]
  fn eval_proc_too_few_args() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2")?;
    let mut ctx = EvalContext::new();
    let result = eval(&ast, &mut ctx);
    assert_matches!(result, Err(EvalError::ArgumentError(_)));
    Ok(())
  }

  #[test]
  fn eval_proc_too_many_args() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2 3 4")?;
    let mut ctx = EvalContext::new();
    let result = eval(&ast, &mut ctx);
    assert_matches!(result, Err(EvalError::ArgumentError(_)));
    Ok(())
  }
}
