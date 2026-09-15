use super::*;
use crate::eval::context::GLOBAL_FRAME;
use crate::eval_error::EvalError;
use crate::parser::{self, CommandNode, ScriptNode, WordNode, WordPart};
use crate::value::Value;
use std::assert_matches;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

#[pollster::test]
async fn eval_set_var_name() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 2")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  let mut val_x = ctx.get_variable(GLOBAL_FRAME, "x").unwrap().clone();
  assert_eq!(val_x.repr_int()?, 2);
  assert_eq!(result.repr_str()?, "2");
  Ok(())
}

#[pollster::test]
async fn eval_set_var() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(&parser::parse("set x 2")?, &mut ctx).await?;
  let mut result = eval(&parser::parse("set x")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_expr_remainder() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("expr {5 % 2}")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_plus_minus() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  assert_eq!(
    eval(&parser::parse("expr {+1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {-1}")?, &mut ctx)
      .await?
      .repr_int()?,
    -1
  );
  assert_eq!(
    eval(&parser::parse("expr {+-1}")?, &mut ctx)
      .await?
      .repr_int()?,
    -1
  );
  assert_eq!(
    eval(&parser::parse("expr {-+1}")?, &mut ctx)
      .await?
      .repr_int()?,
    -1
  );
  assert_eq!(
    eval(&parser::parse("expr {--1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {+ 1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {- 1}")?, &mut ctx)
      .await?
      .repr_int()?,
    -1
  );

  let mut result = eval(&parser::parse("expr {+1.5}")?, &mut ctx).await?;
  assert_eq!(result.repr_float()?, 1.5);
  let mut result = eval(&parser::parse("expr {-1.5}")?, &mut ctx).await?;
  assert_eq!(result.repr_float()?, -1.5);
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_bitwise_not() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  assert_eq!(
    eval(&parser::parse("expr {~0}")?, &mut ctx)
      .await?
      .repr_int()?,
    -1
  );
  assert_eq!(
    eval(&parser::parse("expr {~1}")?, &mut ctx)
      .await?
      .repr_int()?,
    -2
  );
  assert_eq!(
    eval(&parser::parse("expr {~-1}")?, &mut ctx)
      .await?
      .repr_int()?,
    0
  );
  assert_matches!(
    eval(&parser::parse("expr {~1.5}")?, &mut ctx).await,
    Err(EvalError::ArgumentError(_))
  );
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_logical_not() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  assert_eq!(
    eval(&parser::parse("expr {!0}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {!1}")?, &mut ctx)
      .await?
      .repr_int()?,
    0
  );
  assert_eq!(
    eval(&parser::parse("expr {!0.0}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {!0.1}")?, &mut ctx)
      .await?
      .repr_int()?,
    0
  );
  assert_eq!(
    eval(&parser::parse("expr {! 0}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {!!1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {!0 + 1}")?, &mut ctx)
      .await?
      .repr_int()?,
    2
  );

  for (src, expected) in [
    ("expr {!true}", 0),
    ("expr {!false}", 1),
    ("expr {!yes}", 0),
    ("expr {!no}", 1),
    ("expr {!on}", 0),
    ("expr {!off}", 1),
    ("expr {!t}", 0),
    ("expr {!f}", 1),
    ("expr {!tru}", 0),
    ("expr {!fa}", 1),
    ("expr {!n}", 1),
    ("expr {!of}", 1),
  ] {
    assert_eq!(
      eval(&parser::parse(src)?, &mut ctx).await?.repr_int()?,
      expected,
      "{src}"
    );
  }
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_logical_not_rejects_bool_prefixes()
-> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  for src in [
    "expr {!truex}",
    "expr {!yesterday}",
    "expr {!offline}",
    "expr {!falsex}",
    "expr {!o}",
    "expr {!foo}",
  ] {
    assert_matches!(
      eval(&parser::parse(src)?, &mut ctx).await,
      Err(EvalError::ArgumentError(_)),
      "{src}"
    );
  }
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_rejects_non_numeric() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  assert_matches!(
    eval(&parser::parse("expr {+true}")?, &mut ctx).await,
    Err(EvalError::ArgumentError(_))
  );
  assert_matches!(
    eval(&parser::parse("expr {-true}")?, &mut ctx).await,
    Err(EvalError::ArgumentError(_))
  );
  assert_matches!(
    eval(&parser::parse("expr {~true}")?, &mut ctx).await,
    Err(EvalError::ArgumentError(_))
  );
  Ok(())
}

#[pollster::test]
async fn eval_expr_unary_with_binary_precedence() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  assert_eq!(
    eval(&parser::parse("expr {-1 * 2 + 3}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {!~-1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    eval(&parser::parse("expr {! ~ -1}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  Ok(())
}

#[pollster::test]
async fn eval_expr_boolean_and_or() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  for (src, expected) in [
    ("expr {1 && 1}", 1),
    ("expr {1 && 0}", 0),
    ("expr {0 && 1}", 0),
    ("expr {0 && 0}", 0),
    ("expr {0 || 0}", 0),
    ("expr {0 || 1}", 1),
    ("expr {1 || 0}", 1),
    ("expr {1 || 1}", 1),
    ("expr {1 && 0 || 1}", 1),
    ("expr {0 || 1 && 0}", 0),
    ("expr {1 == 1 && 2 < 3}", 1),
    ("expr {1 == 2 || 3 < 4}", 1),
    ("expr {!0 && 1}", 1),
    ("expr {true && yes}", 1),
    ("expr {false || no}", 0),
    ("expr {on && off}", 0),
    ("expr {0.0 || 0.1}", 1),
  ] {
    assert_eq!(
      eval(&parser::parse(src)?, &mut ctx).await?.repr_int()?,
      expected,
      "{src}"
    );
  }
  Ok(())
}

#[pollster::test]
async fn eval_expr_boolean_short_circuit() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();

  // right side must not run when && short-circuits on false left
  eval(&parser::parse("set x 0")?, &mut ctx).await?;
  assert_eq!(
    eval(&parser::parse("expr {0 && [set x 1]}")?, &mut ctx)
      .await?
      .repr_int()?,
    0
  );
  assert_eq!(
    ctx
      .get_variable(GLOBAL_FRAME, "x")
      .unwrap()
      .clone()
      .repr_int()?,
    0
  );

  // right side must not run when || short-circuits on true left
  eval(&parser::parse("set x 0")?, &mut ctx).await?;
  assert_eq!(
    eval(&parser::parse("expr {1 || [set x 1]}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    ctx
      .get_variable(GLOBAL_FRAME, "x")
      .unwrap()
      .clone()
      .repr_int()?,
    0
  );

  // non-short-circuit path still evaluates the right side
  eval(&parser::parse("set x 0")?, &mut ctx).await?;
  assert_eq!(
    eval(&parser::parse("expr {1 && [set x 1]}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    ctx
      .get_variable(GLOBAL_FRAME, "x")
      .unwrap()
      .clone()
      .repr_int()?,
    1
  );

  eval(&parser::parse("set x 0")?, &mut ctx).await?;
  assert_eq!(
    eval(&parser::parse("expr {0 || [set x 1]}")?, &mut ctx)
      .await?
      .repr_int()?,
    1
  );
  assert_eq!(
    ctx
      .get_variable(GLOBAL_FRAME, "x")
      .unwrap()
      .clone()
      .repr_int()?,
    1
  );
  Ok(())
}

#[pollster::test]
async fn eval_expr_boolean_rejects_non_bool() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  for src in ["expr {foo && 1}", "expr {0 || bar}", "expr {1 && truex}"] {
    assert_matches!(
      eval(&parser::parse(src)?, &mut ctx).await,
      Err(EvalError::ArgumentError(_)),
      "{src}"
    );
  }
  Ok(())
}

#[pollster::test]
async fn eval_if_simple() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("if {$x} {expr yes} {expr no}")?;
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 1.into());
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "yes");
  }
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 0.into());
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "no");
  }
  Ok(())
}

#[pollster::test]
async fn eval_if_verbose() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("if {$x} then {expr yes} else {expr no}")?;
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 1.into());
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "yes");
  }
  {
    let mut ctx = EvalContext::new();
    ctx.set_variable(GLOBAL_FRAME, "x", 0.into());
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "no");
  }
  Ok(())
}

#[pollster::test]
async fn eval_if_elseif_one_verbose() -> Result<(), Box<dyn std::error::Error>> {
  {
    let ast = parser::parse("if {0} then {expr 0} elseif {1} then {expr 1} else {expr 2}")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "1");
  }
  {
    let ast = parser::parse("if {0} then {expr 0} elseif {0} then {expr 1} else {expr 2}")?;
    let mut ctx = EvalContext::new();
    let mut result = eval(&ast, &mut ctx).await?;
    assert_eq!(result.repr_str()?, "2");
  }
  Ok(())
}

#[pollster::test]
async fn eval_if_elseif_three_verbose() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse(
    "if {0} then {expr 0} elseif {0} then {expr 1} elseif {1} then {expr 2} else {expr 3}",
  )?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
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

#[pollster::test]
async fn eval_proc_no_args() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc hi {} {expr hey}")?;
  let mut ctx = EvalContext::new();
  eval(&ast, &mut ctx).await?;
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

#[pollster::test]
async fn eval_proc_args_invoke() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2 3")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 6);
  Ok(())
}

#[pollster::test]
async fn eval_proc_local_variables_do_not_leak() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {set x 1}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_does_not_read_globals_by_default() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {expr $x}; f")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
  Ok(())
}

#[pollster::test]
async fn eval_global_reads_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {global x; expr $x}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_global_writes_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {global x; set x 2}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_global_at_top_level_is_noop() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("global x; set x 1; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_global_at_top_level_still_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("global [set name x]; expr $name")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "x");
  Ok(())
}

#[pollster::test]
async fn eval_global_does_not_overwrite_local_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x global; proc f {} {set x local; global x}; f")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn context_resolves_relative_frame_ids() {
  let mut ctx = EvalContext::new();
  ctx
    .run_with_frame(GLOBAL_FRAME, async |ctx, outer| {
      ctx
        .run_with_frame(outer, async |ctx, inner| {
          assert_eq!(ctx.frameid_relative(inner, 0), Some(inner));
          assert_eq!(ctx.frameid_relative(inner, 1), Some(outer));
          assert_eq!(ctx.frameid_relative(inner, 2), Some(GLOBAL_FRAME));
          assert_eq!(ctx.frameid_relative(inner, 3), None);
        })
        .await;
    })
    .await;
}

#[pollster::test]
async fn context_resolves_absolute_frame_ids() {
  let mut ctx = EvalContext::new();
  ctx
    .run_with_frame(GLOBAL_FRAME, async |ctx, outer| {
      ctx
        .run_with_frame(outer, async |ctx, inner| {
          assert_eq!(ctx.frameid_absolute(inner, 0), Some(GLOBAL_FRAME));
          assert_eq!(ctx.frameid_absolute(inner, 1), Some(outer));
          assert_eq!(ctx.frameid_absolute(inner, 2), Some(inner));
          assert_eq!(ctx.frameid_absolute(inner, 3), None);
        })
        .await;
    })
    .await;
}

#[pollster::test]
async fn eval_uplevel_relative_writes_caller_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {uplevel 1 {set x 2}}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_default_level_writes_caller_variable()
-> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {uplevel {set x 2}}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_default_level_reads_caller_variable() -> Result<(), Box<dyn std::error::Error>>
{
  let ast = parser::parse("set x 1; proc f {} {uplevel {expr $x}}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_relative_reads_caller_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {} {uplevel 1 {expr $x}}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_absolute_writes_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {uplevel #0 {set x 3}}; f; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_relative_nested_to_global() -> Result<(), Box<dyn std::error::Error>> {
  let ast =
    parser::parse("proc inner {} {uplevel 2 {set x 4}}; proc outer {} {inner}; outer; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 4);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_relative_nested_to_caller() -> Result<(), Box<dyn std::error::Error>> {
  let ast =
    parser::parse("proc inner {} {uplevel 1 {set y 5}}; proc outer {} {inner; expr $y}; outer")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 5);
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("uplevel")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("uplevel 1")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_uplevel_invalid_level() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(
    &parser::parse("proc f {} {uplevel -1 {expr 1}}; f")?,
    &mut ctx,
  )
  .await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(
    &parser::parse("proc f {} {uplevel 2 {expr 1}}; f")?,
    &mut ctx,
  )
  .await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_upvar_relative_writes_caller_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 1; proc f {name} {upvar 1 $name v; set v 99}; f x; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 99);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_default_level_writes_caller_variable() -> Result<(), Box<dyn std::error::Error>>
{
  let ast = parser::parse("set x 1; proc f {name} {upvar $name v; set v 99}; f x; expr $x")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 99);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_reads_caller_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x 42; proc f {} {upvar 1 x v; expr $v}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 42);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_absolute_links_global_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {upvar #0 g v; set v 7}; f; expr $g")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 7);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_multiple_pairs() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {upvar 1 a x b y; set x 1; set y 2}; f; return \"$a $b\"")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "1 2");
  Ok(())
}

#[pollster::test]
async fn eval_upvar_creates_target_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {upvar 1 newvar v; set v 5}; f; expr $newvar")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 5);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_nested_level_two() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse(
    "set x 0; proc inner {} {upvar 2 x v; set v 8}; proc outer {} {inner}; outer; expr $x",
  )?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 8);
  Ok(())
}

#[pollster::test]
async fn eval_upvar_reading_unset_target_is_undefined() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {} {upvar 1 nope v; expr $v}; f")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
  Ok(())
}

#[pollster::test]
async fn eval_upvar_even_argcount_omits_level() -> Result<(), Box<dyn std::error::Error>> {
  // With an even number of args the level is omitted (defaults to 1), so "1" is
  // the name of the caller's variable and "a" is the local alias. This matches
  // Tcl's argument-count parity rule.
  let ast = parser::parse("proc f {} {upvar 1 a; set a NEW}; f; set 1")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "NEW");
  Ok(())
}

#[pollster::test]
async fn eval_upvar_odd_argcount_requires_valid_level() -> Result<(), Box<dyn std::error::Error>> {
  // With an odd number of args the first arg must be a level; a non-level value
  // is an error rather than being reinterpreted as a variable name.
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("proc f {} {upvar foo x v}; f")?, &mut ctx).await;
  assert_matches!(result, Err(_));
  Ok(())
}

#[pollster::test]
async fn eval_upvar_level_without_pairs() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("proc f {} {upvar 1}; f")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_upvar_no_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("upvar")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_upvar_invalid_level() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(
    &parser::parse("proc f {} {upvar 5 x v; set v 1}; f")?,
    &mut ctx,
  )
  .await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_args_rest_invoke() -> Result<(), Box<dyn std::error::Error>> {
  let ast =
    parser::parse("proc drop_first_two {x y args} {return \"$args\"}; drop_first_two a b c d e")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "c d e");
  Ok(())
}

#[pollster::test]
async fn eval_proc_braced_args_param_is_rest_arg() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc collect {{args}} {return \"$args\"}; collect a b c")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_quoted_word_var_sub() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set name Tcl; set greeting \"hello $name\"")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "hello Tcl");
  Ok(())
}

#[pollster::test]
async fn eval_quoted_word_command_sub() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set greeting \"sum [expr 1 + 2]\"")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "sum 3");
  Ok(())
}

#[pollster::test]
async fn eval_proc_braced_dollar_param_is_literal() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("set x args; proc f {$x} {expr \"$args\"}; f a b")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_too_few_args() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_too_many_args() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc mul {x y} {expr $x * $y}; mul 2 3 4")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_default_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(
    &parser::parse("proc f {x {y 2}} {return \"$x $y\"}; proc g {{a 1} {b 2}} {return \"$a $b\"}")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(
    eval(&parser::parse("f 1")?, &mut ctx).await?.repr_str()?,
    "1 2"
  );
  assert_eq!(
    eval(&parser::parse("f 1 9")?, &mut ctx).await?.repr_str()?,
    "1 9"
  );
  assert_eq!(
    eval(&parser::parse("g")?, &mut ctx).await?.repr_str()?,
    "1 2"
  );
  Ok(())
}

#[pollster::test]
async fn eval_proc_default_then_rest_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(
    &parser::parse("proc f {x {y 1} args} {return \"$x|$y|$args\"}")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(
    eval(&parser::parse("f a")?, &mut ctx).await?.repr_str()?,
    "a|1|"
  );
  assert_eq!(
    eval(&parser::parse("f a b c d")?, &mut ctx)
      .await?
      .repr_str()?,
    "a|b|c d"
  );
  Ok(())
}

#[pollster::test]
async fn eval_proc_default_arg_with_spaces() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {{x {hello world}}} {return $x}; f")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "hello world");
  Ok(())
}

#[pollster::test]
async fn eval_proc_default_arg_too_few() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {x {y 2}} {return \"$x $y\"}; f")?;
  let mut ctx = EvalContext::new();
  let result = eval(&ast, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_proc_too_many_fields_in_arg_specifier() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("proc f {{x 1 2}} {return 1}")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("proc f {{{a}x}} {return 1}")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_return_from_script() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("expr 1; expr 2; return 3; expr 4")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[pollster::test]
async fn eval_return_from_proc() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {x} {return $x}; f 4")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 4);
  Ok(())
}

#[pollster::test]
async fn eval_return_from_proc_deep() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("proc f {x} {if {0 < 1} {return $x}}; f 5")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 5);
  Ok(())
}

#[pollster::test]
async fn eval_info_exists() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(&parser::parse("set x 2")?, &mut ctx).await?;
  let mut result_x = eval(&parser::parse("info exists x")?, &mut ctx).await?;
  let mut result_y = eval(&parser::parse("info exists y")?, &mut ctx).await?;
  assert_eq!(result_x.repr_int()?, 1);
  assert_eq!(result_y.repr_int()?, 0);
  Ok(())
}

#[pollster::test]
async fn eval_dict_create() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("dict create answer 42 greeting {hello world}")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  let dict = result.repr_dict()?;
  let mut answer = dict.get("answer").unwrap().clone();
  assert_eq!(answer.repr_int()?, 42);
  Ok(())
}

#[pollster::test]
async fn eval_dict_get_nested_key() -> Result<(), Box<dyn std::error::Error>> {
  let ast = parser::parse("dict get [dict create a [dict create b {hello world}]] a b")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "hello world");
  Ok(())
}

#[pollster::test]
async fn eval_dict_exists_nested_key() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  eval(
    &parser::parse("set d [dict create a [dict create b 1]]")?,
    &mut ctx,
  )
  .await?;
  let mut present = eval(&parser::parse("dict exists $d a b")?, &mut ctx).await?;
  let mut missing = eval(&parser::parse("dict exists $d a c")?, &mut ctx).await?;
  assert_eq!(present.repr_int()?, 1);
  assert_eq!(missing.repr_int()?, 0);
  Ok(())
}

#[pollster::test]
async fn eval_dict_set_updates_nested_variable() -> Result<(), Box<dyn std::error::Error>> {
  let ast =
    parser::parse("set d [dict create a [dict create b 1]]; dict set d a b 2; dict get $d a b")?;
  let mut ctx = EvalContext::new();
  let mut result = eval(&ast, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_string_index() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string index hello 1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "e");
  Ok(())
}

#[pollster::test]
async fn eval_string_index_unicode() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string index {a🦀b} 1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "🦀");
  Ok(())
}

#[pollster::test]
async fn eval_string_index_negative() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("string index hello -1")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_string_index_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("string index hello 5")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_string_length() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("string length hello")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 5);
  Ok(())
}

#[pollster::test]
async fn eval_list_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[pollster::test]
async fn eval_list_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list a b c")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_list_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list {hello world} x")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "{hello world} x");
  Ok(())
}

#[pollster::test]
async fn eval_list_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; list $x [expr 1 + 2]")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "1 3");
  Ok(())
}

#[pollster::test]
async fn eval_list_nested() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("list [list a b] c")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "{a b} c");
  Ok(())
}

#[pollster::test]
async fn eval_llength_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength [list]")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 0);
  Ok(())
}

#[pollster::test]
async fn eval_llength_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength [list a b c]")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[pollster::test]
async fn eval_llength_from_string() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("llength {a b {c d}}")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 3);
  Ok(())
}

#[pollster::test]
async fn eval_llength_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("llength")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lindex_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b c] 1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "b");
  Ok(())
}

#[pollster::test]
async fn eval_lindex_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex {a {hello world} c} 1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "hello world");
  Ok(())
}

#[pollster::test]
async fn eval_lindex_nested_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list [list a b] c] 0")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lindex_negative() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b] -1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[pollster::test]
async fn eval_lindex_out_of_bounds() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lindex [list a b] 2")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[pollster::test]
async fn eval_lindex_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lindex [list a]")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list a b c]")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "c b a");
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_empty() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list]")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "");
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_nested() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lreverse [list [list a b] c]")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "c {a b}");
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_does_not_mutate_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs [list a b c]; lreverse $xs; set xs")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lreverse $xs")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedVariable(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lreverse_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lreverse")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lappend_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a b]; lappend xs c")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_multiple_values() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a]; lappend xs b c")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_creates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lappend xs a b; set xs")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_mutates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs [list a]; lappend xs b; set xs")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_no_values() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs [list a b]; lappend xs")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_braced_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lappend xs {hello world}")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "{hello world}");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_evals_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set x 1; lappend xs $x [expr 1 + 2]")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "1 3");
  Ok(())
}

#[pollster::test]
async fn eval_lappend_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lappend")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lassign_assigns_in_order_and_returns_rest() -> Result<(), Box<dyn std::error::Error>>
{
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lassign {a b c d} x y")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "c d");
  let mut vars = eval(&parser::parse("list $x $y")?, &mut ctx).await?;
  assert_eq!(vars.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lassign_more_vars_than_elements() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lassign {a} x y z")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "");
  let mut vars = eval(&parser::parse("list $x $y $z")?, &mut ctx).await?;
  assert_eq!(vars.repr_str()?, "a {} {}");
  Ok(())
}

#[pollster::test]
async fn eval_lassign_nested_element() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("lassign {{a b} c} x; set x")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_lassign_assigns_in_local_frame() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set x outer; proc f {} {lassign {inner} x; return $x}; list [f] $x")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "inner outer");
  Ok(())
}

#[pollster::test]
async fn eval_lassign_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lassign")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lset_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs {a b c}; lset xs 1 Z")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "a Z c");
  let mut var = eval(&parser::parse("set xs")?, &mut ctx).await?;
  assert_eq!(var.repr_str()?, "a Z c");
  Ok(())
}

#[pollster::test]
async fn eval_lset_no_indices_replaces_value() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs {a b c}; lset xs {d e}; set xs")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "d e");
  Ok(())
}

#[pollster::test]
async fn eval_lset_nested_index_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs {{a b} {c d}}; lset xs 1 0 Z")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "{a b} {Z d}");
  Ok(())
}

#[pollster::test]
async fn eval_lset_nested_index_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("set xs {{a b} {c d}}; lset xs {1 0} Z")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "{a b} {Z d}");
  Ok(())
}

#[pollster::test]
async fn eval_lset_replaces_element_with_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set xs {a b}; lset xs 0 {x y}")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "{x y} b");
  Ok(())
}

#[pollster::test]
async fn eval_lset_mutates_in_local_frame() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse(
      "set xs {a b}; proc f {} {set xs {c d}; lset xs 0 Z; return $xs}; list [f] $xs",
    )?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "{Z d} {a b}");
  Ok(())
}

#[pollster::test]
async fn eval_lset_index_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("set xs {a b}; lset xs 2 Z")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lset_negative_index() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("set xs {a b}; lset xs -1 Z")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lset_index_into_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lset xs 0 Z")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_lset_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("lset xs")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_incr_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; incr x")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 2);
  Ok(())
}

#[pollster::test]
async fn eval_incr_with_increment() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("set x 1; incr x 5")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 6);
  Ok(())
}

#[pollster::test]
async fn eval_incr_creates_variable() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(&parser::parse("incr x; set x")?, &mut ctx).await?;
  assert_eq!(result.repr_int()?, 1);
  Ok(())
}

#[pollster::test]
async fn eval_incr_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("incr")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("incr x 1 2")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_foreach_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {a b c} {lappend out $x}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "a b c");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_multi_var() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach {a b} {1 2 3 4} {lappend out \"$a$b\"}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "12 34");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_multi_varlist() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {a b} y {1 2} {lappend out \"$x$y\"}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "a1 b2");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_wrong_arity() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("foreach")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  let result = eval(&parser::parse("foreach x {a b}")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_foreach_mutates_containing_scope() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc f {} {foreach x {a b c} {incr n}; return \"$n $x\"}; f")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "3 c");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_empty_varlist() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("foreach {} {a b} {}")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_foreach_break() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {1 2 3 4} {lappend out $x; if {$x == 2} {break}}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "1 2");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_continue() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach x {1 2 3 4} {if {$x == 2} {continue}; lappend out $x}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "1 3 4");
  Ok(())
}

#[pollster::test]
async fn eval_while_continue() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse(
      "set i 0; while {$i < 4} {incr i; if {$i == 2} {continue}; lappend out $i}; set out",
    )?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "1 3 4");
  Ok(())
}

#[pollster::test]
async fn eval_foreach_uneven_list() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("foreach {a b} {1 2 3} {lappend out \"$a$b\"}; set out")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "12 3");
  Ok(())
}

fn context_with_test_clock() -> EvalContext {
  let clock_monotonic = Rc::new(Cell::new(Duration::ZERO));
  let event_clock = clock_monotonic.clone();
  let sleep_clock = clock_monotonic.clone();

  EvalContext::new()
    .with_clock_monotonic(move || event_clock.get())
    .with_sleep_ms(move |ms| {
      sleep_clock.set(sleep_clock.get() + Duration::from_millis(ms));
      async { Ok(()) }
    })
}

#[pollster::test]
async fn eval_update_does_not_fire_future_timer() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = context_with_test_clock();
  eval(&parser::parse("after 10 {set future 1}; update")?, &mut ctx).await?;

  assert!(ctx.get_variable(GLOBAL_FRAME, "future").is_none());
  assert_eq!(ctx.count_pending_events(), 1);
  Ok(())
}

#[pollster::test]
async fn eval_vwait_ignores_unrelated_events() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = context_with_test_clock();
  eval(
    &parser::parse(
      "set watched 0; after 10 {set other 1}; after 20 {set watched 1}; vwait watched",
    )?,
    &mut ctx,
  )
  .await?;

  assert_eq!(
    ctx
      .get_variable(GLOBAL_FRAME, "other")
      .unwrap()
      .clone()
      .repr_int()?,
    1
  );
  Ok(())
}

#[pollster::test]
async fn eval_vwait_leaves_later_events_pending() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = context_with_test_clock();
  eval(
    &parser::parse(
      "set watched 0; after 10 {set watched 1}; after 20 {set later 1}; vwait watched",
    )?,
    &mut ctx,
  )
  .await?;

  assert!(ctx.get_variable(GLOBAL_FRAME, "later").is_none());
  assert_eq!(ctx.count_pending_events(), 1);
  Ok(())
}

#[pollster::test]
async fn eval_vwait_detects_same_value_write_through_alias()
-> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = context_with_test_clock();
  eval(
    &parser::parse(
      "set watched 0; upvar #0 watched alias; after 10 {set watched 0}; after 20 {set later 1}; vwait alias",
    )?,
    &mut ctx,
  )
  .await?;

  assert!(ctx.get_variable(GLOBAL_FRAME, "later").is_none());
  assert_eq!(ctx.count_pending_events(), 1);
  Ok(())
}

#[pollster::test]
async fn eval_vwait_watches_global_variable_from_proc() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = context_with_test_clock();
  let mut result = eval(
    &parser::parse(
      "proc wait {} {set watched local; after 10 {set watched global}; after 20 {set later 1}; vwait watched; return $watched}; wait",
    )?,
    &mut ctx,
  )
  .await?;

  assert_eq!(result.repr_str()?, "local");
  assert!(ctx.get_variable(GLOBAL_FRAME, "later").is_none());
  assert_eq!(ctx.count_pending_events(), 1);
  Ok(())
}

#[pollster::test]
async fn eval_undefined_command() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("bogus 1 2")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedCommand(name)) if name == "bogus");
  Ok(())
}

#[pollster::test]
async fn eval_unknown_invoke() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("unknown bogus 1 2")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedCommand(name)) if name == "bogus");
  Ok(())
}

#[pollster::test]
async fn eval_unknown_no_args() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let result = eval(&parser::parse("unknown")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::ArgumentError(_)));
  Ok(())
}

#[pollster::test]
async fn eval_unknown_proc_receives_command_name() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return \"$args\"}; bogus a b")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "bogus a b");
  Ok(())
}

#[pollster::test]
async fn eval_unknown_proc_invoked_directly() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return \"$args\"}; unknown a b")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "a b");
  Ok(())
}

#[pollster::test]
async fn eval_unknown_proc_not_used_for_defined_commands() -> Result<(), Box<dyn std::error::Error>>
{
  let mut ctx = EvalContext::new();
  let mut result = eval(
    &parser::parse("proc unknown {args} {return oops}; proc f {} {return ok}; f; set x [expr 1]")?,
    &mut ctx,
  )
  .await?;
  assert_eq!(result.repr_str()?, "1");
  Ok(())
}

#[pollster::test]
async fn eval_register_command() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  ctx.register_command("hello", |_args, _ctx, _frame| Ok(Value::new("hi")));
  let mut result = eval(&parser::parse("hello")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "hi");
  Ok(())
}

#[pollster::test]
async fn eval_unregister_command() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  ctx.unregister_command("set");
  let result = eval(&parser::parse("set x 1")?, &mut ctx).await;
  assert_matches!(result, Err(EvalError::UndefinedCommand(name)) if name == "set");
  Ok(())
}

#[pollster::test]
async fn eval_register_command_overrides_builtin() -> Result<(), Box<dyn std::error::Error>> {
  let mut ctx = EvalContext::new();
  ctx.register_command("set", |_args, _ctx, _frame| Ok(Value::new("overridden")));
  let mut result = eval(&parser::parse("set x 1")?, &mut ctx).await?;
  assert_eq!(result.repr_str()?, "overridden");
  Ok(())
}
