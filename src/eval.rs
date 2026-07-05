use crate::eval_error::EvalError;
use crate::parser;
use crate::parser::{CommandNode, ScriptNode, WordNode, WordPart};
use crate::parser_expr;
use crate::parser_expr::{BinaryOp, ExprNode};
use crate::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Clone, Debug)]
pub struct Proc {
  params: Vec<String>,
  body: ScriptNode,
}

type FrameId = usize;
const GLOBAL_FRAME: FrameId = 0;

#[derive(Clone, Debug)]
pub struct EvalContext {
  procs: HashMap<String, Rc<Proc>>,
  frames: Vec<EvalFrame>,
  expr_cache: HashMap<String, Rc<ExprNode>>,
  script_cache: HashMap<String, Rc<ScriptNode>>,
}

#[derive(Clone, Debug)]
pub struct EvalFrame {
  caller: Option<FrameId>,
  variables: HashMap<String, Value>,
}

impl EvalContext {
  pub fn new() -> EvalContext {
    EvalContext {
      procs: HashMap::new(),
      frames: vec![EvalFrame::new()],
      expr_cache: HashMap::new(),
      script_cache: HashMap::new(),
    }
  }

  /// Parse an expression, caching the resulting AST by source string so that
  /// repeated evaluations (e.g. a condition inside a recursive proc) parse once.
  pub fn parse_expr_cached(&mut self, src: &str) -> Result<Rc<ExprNode>, EvalError> {
    if let Some(node) = self.expr_cache.get(src) {
      return Ok(node.clone());
    }
    let (node, _) = parser_expr::parse_expr(src)
      .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
    let node = Rc::new(node);
    self.expr_cache.insert(src.to_string(), node.clone());
    Ok(node)
  }

  /// Parse a script, caching the resulting AST by source string so that
  /// repeated evaluations (e.g. a proc/if body) parse once.
  pub fn parse_script_cached(&mut self, src: &str) -> Result<Rc<ScriptNode>, EvalError> {
    if let Some(node) = self.script_cache.get(src) {
      return Ok(node.clone());
    }
    let node = parser::parse(src).map_err(|e| EvalError::ScriptParseError(e.to_string()))?;
    let node = Rc::new(node);
    self.script_cache.insert(src.to_string(), node.clone());
    Ok(node)
  }

  pub fn frame(&self, id: FrameId) -> &EvalFrame {
    self.frames.get(id).unwrap()
  }

  pub fn frame_mut(&mut self, id: FrameId) -> &mut EvalFrame {
    self.frames.get_mut(id).unwrap()
  }

  pub fn run_with_frame<R>(
    &mut self,
    calling_frame: FrameId,
    f: impl FnOnce(&mut EvalContext, FrameId) -> R,
  ) -> R {
    let next_id = self.frames.len();
    self.frames.push(EvalFrame::new_from(calling_frame));
    let result = f(self, next_id);
    self.frames.pop();
    result
  }

  pub fn get_proc(&self, name: &str) -> Option<Rc<Proc>> {
    self.procs.get(name).cloned()
  }

  pub fn set_proc(&mut self, name: &str, proc: Proc) {
    self.procs.insert(name.to_string(), Rc::new(proc));
  }
}

impl EvalFrame {
  pub fn new() -> EvalFrame {
    EvalFrame {
      caller: None,
      variables: HashMap::new(),
    }
  }

  pub fn new_from(frame: FrameId) -> EvalFrame {
    EvalFrame {
      caller: Some(frame),
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
  eval_returnable_script(script, context, GLOBAL_FRAME)
}

pub fn eval_returnable_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    match eval_command(&command, context, frame) {
      Ok(val) => result = val,
      Err(EvalError::ReturnError(val)) => {
        result = val;
        break;
      }
      err => return err,
    }
  }
  Ok(result)
}

pub fn eval_script(
  script: &ScriptNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut result = Value::none();
  for command in &script.commands {
    result = eval_command(&command, context, frame)?;
  }
  Ok(result)
}

pub fn eval_command(
  command: &CommandNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let [name, args @ ..] = command.words.as_slice() else {
    return Err(EvalError::Generic("missing command name".to_string()));
  };

  let mut name_value = eval_word(&name, context, frame)?;
  let name_str = name_value.repr_str()?;

  // user-defined proc
  if let Some(proc) = context.get_proc(name_str) {
    return eval_proc(name_str, &proc, args, context, frame);
  }

  // builtin
  match name_str {
    "break" => eval_cmd_break(args, context, frame),
    "expr" => eval_cmd_expr(args, context, frame),
    "if" => eval_cmd_if(args, context, frame),
    "proc" => eval_cmd_proc(args, context, frame),
    "puts" => eval_cmd_puts(args, context, frame),
    "return" => eval_cmd_return(args, context, frame),
    "set" => eval_cmd_set(args, context, frame),
    "while" => eval_cmd_while(args, context, frame),
    _ => Err(EvalError::UndefinedCommand(name_str.to_string())),
  }
}

pub fn eval_cmd_break(
  words: &[WordNode],
  _context: &mut EvalContext,
  _frame: FrameId,
) -> Result<Value, EvalError> {
  if !words.is_empty() {
    return Err(EvalError::ArgumentError(
      "break expects no arguments".to_string(),
    ));
  }
  Err(EvalError::BreakError)
}

pub fn eval_cmd_expr(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let values = words
    .iter()
    .map(|word| eval_word(&word, context, frame).map(|value| value.to_string()));
  let joined = values.collect::<Result<Vec<String>, _>>()?.join(" ");
  let node = context.parse_expr_cached(joined.as_str())?;
  eval_expr(&node, context, frame)
}

pub fn eval_cmd_if(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut args = words
    .iter()
    .map(|w| eval_word(w, context, frame))
    .peekable();

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
    let cond_parsed = context.parse_expr_cached(cond.repr_str()?)?;
    if eval_expr(&cond_parsed, context, frame)?.repr_int()? != 0 {
      let body_parsed = context.parse_script_cached(body.repr_str()?)?;
      return eval_script(&body_parsed, context, frame);
    }
  }

  Ok(Value::none())
}

pub fn eval_cmd_proc(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let (mut name_val, mut params_val, mut body_val) = match words {
    [name, params, body] => (
      eval_word(name, context, frame)?,
      eval_word(params, context, frame)?,
      eval_word(body, context, frame)?,
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

pub fn eval_cmd_puts(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let [mut string] = match words {
    [_, _, _] => todo!(),
    [_, _] => todo!(),
    [string] => [eval_word(string, context, frame)?],
    [..] => {
      return Err(EvalError::Generic(
        "too many arguments; expects string".to_string(),
      ));
    }
  };

  println!("{}", string.repr_str()?);
  Ok(Value::none())
}

pub fn eval_cmd_return(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  match words.get(0).map(|w| eval_word(w, context, frame)) {
    Some(Ok(val)) => Err(EvalError::ReturnError(val)),
    Some(Err(e)) => Err(e),
    None => Err(EvalError::ReturnError(Value::none())),
  }?
}

pub fn eval_cmd_set(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
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

  let mut name = eval_word(&name, context, frame)?;
  let value = eval_word(&value, context, frame)?;
  context
    .frame_mut(frame)
    .set_variable(name.repr_str()?, value.clone());
  Ok(value)
}

pub fn eval_cmd_while(
  words: &[WordNode],
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let [test, body] = words else {
    return Err(EvalError::Generic(
      "while requires two arguments: test and body".to_string(),
    ));
  };

  let mut test_val = eval_word(test, context, frame)?;
  let test_expr = context.parse_expr_cached(test_val.repr_str()?)?;

  let mut body_val = eval_word(body, context, frame)?;
  let body_script = context.parse_script_cached(body_val.repr_str()?)?;

  while eval_expr(&test_expr, context, frame)?.repr_int()? != 0 {
    match eval_script(&body_script, context, frame) {
      Err(EvalError::BreakError) => break,
      Err(e) => return Err(e),
      Ok(_) => {}
    }
  }

  Ok(Value::none())
}

pub fn eval_expr(
  node: &ExprNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  use ExprNode::*;
  match node {
    Word(w) => eval_word(w, context, frame),
    UnaryOp(_o, _x) => todo!(),
    BinaryOp(o, a, b) => eval_expr_binary_op(o, a.as_ref(), b.as_ref(), context, frame),
    Ternary(_c, _i, _e) => todo!(),
  }
}

pub fn eval_expr_binary_op(
  o: &BinaryOp,
  a: &ExprNode,
  b: &ExprNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  use BinaryOp::*;
  let mut a = eval_expr(a, context, frame)?;
  let mut b = eval_expr(b, context, frame)?;
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
  frame: FrameId,
) -> Result<Value, EvalError> {
  context.run_with_frame(frame, |context, proc_frame| {
    // bind arguments
    let mut args_it = args.iter();
    for (i, param) in proc.params.iter().enumerate() {
      // handle rest args
      if i == proc.params.len() - 1 {
        if param == "args" {
          let args_concat = args_it
            .by_ref()
            .map(|word| {
              eval_word(word, context, frame)?
                .repr_str()
                .map(|str| str.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(" ");
          context
            .frame_mut(proc_frame)
            .set_variable("args", Value::new(args_concat));
          break;
        }
      }

      let value = args_it
        .next()
        .map(|w| eval_word(w, context, frame))
        .ok_or_else(|| EvalError::ArgumentError(format!("not enough args for {}", name)))??;
      context.frame_mut(proc_frame).set_variable(param, value);
    }

    if args_it.next().is_some() {
      return Err(EvalError::ArgumentError(format!(
        "too many args for {}",
        name
      )));
    }

    eval_returnable_script(&proc.body, context, proc_frame)
  })
}

pub fn eval_word(
  word: &WordNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let mut joined = String::new();
  for part in &word.parts {
    let mut value = eval_wordpart(part, context, frame)?;
    joined.push_str(value.repr_str()?);
  }
  Ok(Value::new(joined))
}

pub fn eval_wordpart(
  part: &WordPart,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  match part {
    WordPart::BareLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedLiteral(s) => Ok(Value::new(s)),
    WordPart::BracedSub(v) => context
      .frame(frame)
      .get_variable(&v)
      .ok_or_else(|| EvalError::UndefinedVariable(v.to_string()))
      .cloned(),
    WordPart::CommandSub(c) => {
      let script = context.parse_script_cached(c)?;
      eval_script(&script, context, frame)
    }
    WordPart::Quoted(parts) => {
      let mut result: String = "".to_string();
      for part in parts {
        let mut string = eval_wordpart(part, context, frame)?;
        result.push_str(string.repr_str()?);
      }
      Ok(Value::new(result))
    }
    WordPart::VarSub(v) => context
      .frame(frame)
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
      ctx.frame_mut(GLOBAL_FRAME).set_variable("x", 1.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "yes");
    }
    {
      let mut ctx = EvalContext::new();
      ctx.frame_mut(GLOBAL_FRAME).set_variable("x", 0.into());
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
      ctx.frame_mut(GLOBAL_FRAME).set_variable("x", 1.into());
      let mut result = eval(&ast, &mut ctx)?;
      assert_eq!(result.repr_str()?, "yes");
    }
    {
      let mut ctx = EvalContext::new();
      ctx.frame_mut(GLOBAL_FRAME).set_variable("x", 0.into());
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
      Some(Rc::new(Proc {
        params: vec![],
        body: ScriptNode {
          commands: vec![CommandNode {
            words: vec![lit!("expr"), lit!("hey")]
          }]
        }
      }))
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
  fn eval_proc_local_variables_do_not_leak() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc f {} {set x 1}; f; expr $x")?;
    let mut ctx = EvalContext::new();
    let result = eval(&ast, &mut ctx);
    assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
    Ok(())
  }

  #[test]
  fn eval_proc_does_not_read_globals_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("set x 1; proc f {} {expr $x}; f")?;
    let mut ctx = EvalContext::new();
    let result = eval(&ast, &mut ctx);
    assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
    Ok(())
  }

  #[test]
  fn eval_proc_args_rest_invoke() -> Result<(), Box<dyn std::error::Error>> {
    let ast =
      parser::parse("proc drop_first_two {x y args} {return \"$args\"}; drop_first_two a b c d e")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "c d e");
    Ok(())
  }

  #[test]
  fn eval_proc_braced_args_param_is_rest_arg() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc collect {{args}} {return \"$args\"}; collect a b c")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "a b c");
    Ok(())
  }

  #[test]
  fn eval_quoted_word_var_sub() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("set name Tcl; set greeting \"hello $name\"")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "hello Tcl");
    Ok(())
  }

  #[test]
  fn eval_quoted_word_command_sub() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("set greeting \"sum [expr 1 + 2]\"")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "sum 3");
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

  #[test]
  fn eval_return_from_script() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("expr 1; expr 2; return 3; expr 4")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_int()?, 3);
    Ok(())
  }

  #[test]
  fn eval_return_from_proc() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc f {x} {return $x}; f 4")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_int()?, 4);
    Ok(())
  }

  #[test]
  fn eval_return_from_proc_deep() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("proc f {x} {if {0 < 1} {return $x}}; f 5")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_int()?, 5);
    Ok(())
  }
}
