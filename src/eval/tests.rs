use super::*;
use crate::eval::context::GLOBAL_FRAME;
use crate::eval_error::EvalError;
use crate::parser::{self, CommandNode, ScriptNode, WordNode, WordPart};
use std::assert_matches;

#[test]
fn eval_set_var_name() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 2")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  let mut val_x = ctx.get_variable(GLOBAL_FRAME, "x").unwrap().clone();
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
fn eval_expr_remainder() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("expr {5 % 2}")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[test]
fn eval_if_simple() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("if {$x} {expr yes} {expr no}")?;
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 1.into());
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "yes");
  }
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 0.into());
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
    ctx.set_variable(GLOBAL_FRAME, "x", 1.into());
    let mut result = eval(&ast, &mut ctx)?;
    assert_eq!(result.repr_str()?, "yes");
  }
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 0.into());
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
fn eval_global_reads_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {global x; expr $x}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[test]
fn eval_global_writes_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {global x; set x 2}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[test]
fn eval_global_at_top_level_is_noop() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("global x; set x 1; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[test]
fn eval_global_at_top_level_still_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("global [set name x]; expr $name")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_str()?, "x");
  Ok(())
}

#[test]
fn eval_global_does_not_overwrite_local_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x global; proc f {} {set x local; global x}; f")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
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

#[test]
fn eval_info_exists() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(&parser::parse("set x 2")?, &mut ctx)?;
  let mut result_x = eval(&parser::parse("info exists x")?, &mut ctx)?;
  let mut result_y = eval(&parser::parse("info exists y")?, &mut ctx)?;
  assert_eq!(result_x.repr_int()?, 1);
  assert_eq!(result_y.repr_int()?, 0);
  Ok(())
}

#[test]
fn eval_dict_create() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("dict create answer 42 greeting {hello world}")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  let dict = result.repr_dict()?;
  let mut answer = dict.get("answer").unwrap().clone();
  assert_eq!(answer.repr_int()?, 42);
  Ok(())
}

#[test]
fn eval_dict_get_nested_key() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("dict get [dict create a [dict create b {hello world}]] a b")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_str()?, "hello world");
  Ok(())
}

#[test]
fn eval_dict_exists_nested_key() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(
    &parser::parse("set d [dict create a [dict create b 1]]")?,
    &mut ctx,
  )?;
  let mut present = eval(&parser::parse("dict exists $d a b")?, &mut ctx)?;
  let mut missing = eval(&parser::parse("dict exists $d a c")?, &mut ctx)?;
  assert_eq!(present.repr_int()?, 1);
  assert_eq!(missing.repr_int()?, 0);
  Ok(())
}

#[test]
fn eval_dict_set_updates_nested_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse(
    "set d [dict create a [dict create b 1]]; dict set d a b 2; dict get $d a b",
  )?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx)?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[test]
fn eval_string_index() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string index hello 1")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "e");
  Ok(())
}

#[test]
fn eval_string_index_unicode() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string index {a🦀b} 1")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "🦀");
  Ok(())
}

#[test]
fn eval_string_index_negative() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("string index hello -1")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_string_index_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("string index hello 5")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_string_length() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string length hello")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 5);
  Ok(())
}

#[test]
fn eval_list_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[test]
fn eval_list_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list a b c")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[test]
fn eval_list_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list {hello world} x")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "{hello world} x");
  Ok(())
}

#[test]
fn eval_list_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; list $x [expr 1 + 2]")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "1 3");
  Ok(())
}

#[test]
fn eval_list_nested() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list [list a b] c")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "{a b} c");
  Ok(())
}

#[test]
fn eval_llength_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength [list]")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 0);
  Ok(())
}

#[test]
fn eval_llength_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength [list a b c]")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[test]
fn eval_llength_from_string() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength {a b {c d}}")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[test]
fn eval_llength_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("llength")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_lindex_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b c] 1")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "b");
  Ok(())
}

#[test]
fn eval_lindex_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex {a {hello world} c} 1")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "hello world");
  Ok(())
}

#[test]
fn eval_lindex_nested_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list [list a b] c] 0")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[test]
fn eval_lindex_negative() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b] -1")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[test]
fn eval_lindex_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b] 2")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[test]
fn eval_lindex_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lindex [list a]")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_lreverse_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list a b c]")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "c b a");
  Ok(())
}

#[test]
fn eval_lreverse_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list]")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[test]
fn eval_lreverse_nested() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list [list a b] c]")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "c {a b}");
  Ok(())
}

#[test]
fn eval_lreverse_does_not_mutate_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs [list a b c]; lreverse $xs; set xs")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[test]
fn eval_lreverse_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lreverse $xs")?, &mut ctx);
  assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
  Ok(())
}

#[test]
fn eval_lreverse_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lreverse")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_lappend_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a b]; lappend xs c")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[test]
fn eval_lappend_multiple_values() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a]; lappend xs b c")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[test]
fn eval_lappend_creates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lappend xs a b; set xs")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[test]
fn eval_lappend_mutates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs [list a]; lappend xs b; set xs")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[test]
fn eval_lappend_no_values() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a b]; lappend xs")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[test]
fn eval_lappend_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lappend xs {hello world}")?, &mut ctx)?;
  assert_eq!(result.repr_str()?, "{hello world}");
  Ok(())
}

#[test]
fn eval_lappend_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set x 1; lappend xs $x [expr 1 + 2]")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "1 3");
  Ok(())
}

#[test]
fn eval_lappend_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lappend")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_incr_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; incr x")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[test]
fn eval_incr_with_increment() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; incr x 5")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 6);
  Ok(())
}

#[test]
fn eval_incr_creates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("incr x; set x")?, &mut ctx)?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[test]
fn eval_incr_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("incr")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("incr x 1 2")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_foreach_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {a b c} {lappend out $x}; set out")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[test]
fn eval_foreach_multi_var() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach {a b} {1 2 3 4} {lappend out \"$a$b\"}; set out")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "12 34");
  Ok(())
}

#[test]
fn eval_foreach_multi_varlist() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {a b} y {1 2} {lappend out \"$x$y\"}; set out")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "a1 b2");
  Ok(())
}

#[test]
fn eval_foreach_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("foreach")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("foreach x {a b}")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_foreach_mutates_containing_scope() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc f {} {foreach x {a b c} {incr n}; return \"$n $x\"}; f")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "3 c");
  Ok(())
}

#[test]
fn eval_foreach_empty_varlist() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("foreach {} {a b} {}")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_foreach_break() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {1 2 3 4} {lappend out $x; if {$x == 2} {break}}; set out")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "1 2");
  Ok(())
}

#[test]
fn eval_foreach_continue() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse(
      "foreach x {1 2 3 4} {if {$x == 2} {continue}; lappend out $x}; set out",
    )?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "1 3 4");
  Ok(())
}

#[test]
fn eval_while_continue() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse(
      "set i 0; while {$i < 4} {incr i; if {$i == 2} {continue}; lappend out $i}; set out",
    )?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "1 3 4");
  Ok(())
}

#[test]
fn eval_foreach_uneven_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach {a b} {1 2 3} {lappend out \"$a$b\"}; set out")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "12 3");
  Ok(())
}

#[test]
fn eval_undefined_command() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("bogus 1 2")?, &mut ctx);
  assert_matches!(result, Err(EvalError::UndefinedCommand(name)) if name == "bogus");
  Ok(())
}

#[test]
fn eval_unknown_invoke() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("unknown bogus 1 2")?, &mut ctx);
  assert_matches!(result, Err(EvalError::UndefinedCommand(name)) if name == "bogus");
  Ok(())
}

#[test]
fn eval_unknown_no_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("unknown")?, &mut ctx);
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[test]
fn eval_unknown_proc_receives_command_name() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return \"$args\"}; bogus a b")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "bogus a b");
  Ok(())
}

#[test]
fn eval_unknown_proc_invoked_directly() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return \"$args\"}; unknown a b")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[test]
fn eval_unknown_proc_not_used_for_defined_commands() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return oops}; proc f {} {return ok}; f; set x [expr 1]")?,
    &mut ctx,
  )?;
  assert_eq!(result.repr_str()?, "1");
  Ok(())
}
