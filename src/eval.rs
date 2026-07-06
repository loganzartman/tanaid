use crate::eval_error::EvalError;
use crate::parser;
use crate::parser::{CommandNode, ParseError, ScriptNode, WordNode, WordPart};
use crate::parser_expr;
use crate::parser_expr::{BinaryOp, ExprNode};
use crate::value::Value;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
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
  parse_cache_script: LruCache<String, Rc<(ScriptNode, String)>>,
  parse_cache_expr: LruCache<String, Rc<(ExprNode, String)>>,
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
      parse_cache_script: LruCache::new(NonZeroUsize::new(1024).unwrap()),
      parse_cache_expr: LruCache::new(NonZeroUsize::new(1024).unwrap()),
    }
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

  pub fn parse_script_caching(
    &mut self,
    src: &str,
  ) -> Result<Rc<(ScriptNode, String)>, ParseError> {
    if !self.parse_cache_script.contains(src) {
      let (node, rest) = parser::parse_script(src)?;
      self
        .parse_cache_script
        .put(src.to_string(), Rc::new((node, rest.to_string())));
    }
    Ok(self.parse_cache_script.get(src).unwrap().clone())
  }

  pub fn parse_expr_caching(&mut self, src: &str) -> Result<Rc<(ExprNode, String)>, ParseError> {
    if !self.parse_cache_expr.contains(src) {
      let (node, rest) = parser_expr::parse_expr(src)?;
      self
        .parse_cache_expr
        .put(src.to_string(), Rc::new((node, rest.to_string())));
    }
    Ok(self.parse_cache_expr.get(src).unwrap().clone())
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

  pub fn get_variable_mut(&mut self, name: &str) -> Option<&mut Value> {
    self.variables.get_mut(name)
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
  // TODO: reference-count procs
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
  let expr_src = if let [word] = words
    && let [WordPart::BracedLiteral(braced_src)] = word.parts.as_slice()
  {
    // optimization: no allocation for idiomatic single braced argument
    braced_src
  } else {
    let values = words
      .iter()
      .map(|word| eval_word(&word, context, frame).map(|value| value.to_string()));
    &values.collect::<Result<Vec<String>, _>>()?.join(" ")
  };

  let expr_parsed = context
    .parse_expr_caching(expr_src.as_str())
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (node, _) = expr_parsed.as_ref();

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
    let cond_parse_result = context
      .parse_expr_caching(cond.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if condition: {}", e)))?;
    let (cond_parsed, _) = cond_parse_result.as_ref();

    let body_parse_result = context
      .parse_script_caching(body.repr_str()?)
      .map_err(|e| EvalError::ArgumentError(format!("Failed to parse if body: {}", e)))?;
    let (body_parsed, _) = body_parse_result.as_ref();

    if eval_expr(&cond_parsed, context, frame)?.repr_int()? != 0 {
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

  let parsed = context
    .parse_script_caching(body_val.repr_str()?)
    .map_err(|e| EvalError::ArgumentError(format!("proc body must be a script: {}", e)))?;
  let (body, rest) = parsed.as_ref();

  if !rest.is_empty() {
    return Err(EvalError::ArgumentError(
      "proc body must be a script: trailing input".to_string(),
    ));
  };

  let body = body.clone();
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
  let (name, maybe_value) = match words {
    [name, value] => (name, Some(value)),
    [name] => (name, None),
    [] => return Err(EvalError::Generic("missing variable name".to_string())),
    _ => {
      return Err(EvalError::Generic(
        "too many arguments; expects: set name ?value?".to_string(),
      ));
    }
  };

  let mut name = eval_word(&name, context, frame)?;
  if let Some(value) = maybe_value {
    let value = eval_word(&value, context, frame)?;
    context
      .frame_mut(frame)
      .set_variable(name.repr_str()?, value.clone());
    Ok(value)
  } else {
    Ok(
      context
        .frame(frame)
        .get_variable(name.repr_str()?)
        .ok_or_else(|| EvalError::UndefinedVariable(format!("{}", name.to_string())))?
        .clone(),
    )
  }
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

  let mut expr_src = eval_word(test, context, frame)?;
  let expr_parsed = context
    .parse_expr_caching(expr_src.repr_str()?)
    .map_err(|e| EvalError::ExprParseError(e.to_string()))?;
  let (test_expr, _) = expr_parsed.as_ref();

  let mut body_src = eval_word(body, context, frame)?;
  let body_parsed = context
    .parse_script_caching(body_src.repr_str()?)
    .map_err(|e| EvalError::ScriptParseError(e.to_string()))?;
  let (body_script, _) = body_parsed.as_ref();

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
  // optimization for single-part words
  if let [part] = word.parts.as_slice() {
    return eval_wordpart(part, context, frame);
  }

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
      let parsed = context
        .parse_script_caching(c)
        .map_err(|e| EvalError::CommandParseError(e.to_string()))?;
      let (script, _) = parsed.as_ref();
      eval_script(script, context, frame)
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
  fn eval_set_var_name() -> Result<(), Box<dyn std::error::Error>> {
    let ast = parser::parse("set x 2")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx)?;
    let mut val_x = ctx.frame(GLOBAL_FRAME).get_variable("x").unwrap().clone();
    assert_eq!(val_x.repr_int()?, 2);
    assert_eq!(result.repr_str()?, "2");
    Ok(())
  }

  #[test]
  fn eval_set_var() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = EvalContext::new();
    eval(&parser::parse("set x 2")?, &mut ctx)?;
    let mut result = eval(&parser::parse("set x")?, &mut ctx)?;
    assert_eq!(result.repr_int()?, 2);
    Ok(())
  }

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
      ctx.get_proc("hi").as_deref(),
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
