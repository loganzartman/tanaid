use super::{EvalContext, FrameId, eval_word};
use crate::eval::script::eval_command_inner;
use crate::eval_error::EvalError;
use crate::parser_expr::{BinaryOp, ExprNode, UnaryOp};
use crate::value::Value;

pub async fn eval_expr(
  node: &ExprNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  use ExprNode::*;
  match node {
    Word(w) => eval_word(w, context, frame).await,
    UnaryOp(o, x) => Box::pin(eval_expr_unary_op(o, x.as_ref(), context, frame)).await,
    BinaryOp(o, a, b) => {
      Box::pin(eval_expr_binary_op(
        o,
        a.as_ref(),
        b.as_ref(),
        context,
        frame,
      ))
      .await
    }
    Ternary(_c, _i, _e) => todo!(),
    FnCall(func, args) => Box::pin(eval_expr_fncall(func, args, context, frame)).await,
  }
}

pub async fn eval_expr_unary_op(
  o: &UnaryOp,
  x: &ExprNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  use UnaryOp::*;
  let mut x = eval_expr(x, context, frame).await?;
  match o {
    Plus => x.unary_plus(),
    Minus => -x,
    BitwiseNot => x.bit_not(),
    LogicalNot => !x,
  }
}

pub async fn eval_expr_binary_op(
  o: &BinaryOp,
  a: &ExprNode,
  b: &ExprNode,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  use BinaryOp::*;

  match o {
    And => {
      let mut a = eval_expr(a, context, frame).await?;
      if !a.repr_bool()? {
        return Ok(Value::from(false));
      }
      let mut b = eval_expr(b, context, frame).await?;
      return Ok(Value::from(b.repr_bool()?));
    }
    Or => {
      let mut a = eval_expr(a, context, frame).await?;
      if a.repr_bool()? {
        return Ok(Value::from(true));
      }
      let mut b = eval_expr(b, context, frame).await?;
      return Ok(Value::from(b.repr_bool()?));
    }
    _ => {}
  }

  let mut a = eval_expr(a, context, frame).await?;
  let mut b = eval_expr(b, context, frame).await?;
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
    Rem => a % b,
    And | Or => unreachable!(),
  }
}

pub async fn eval_expr_fncall(
  func: &String,
  args: &Vec<ExprNode>,
  context: &mut EvalContext,
  frame: FrameId,
) -> Result<Value, EvalError> {
  let resolved_name = format!("::tcl::mathfunc::{}", func);
  let mut name_and_args = Vec::with_capacity(args.len());
  name_and_args.push(Value::from(resolved_name));
  for arg in args {
    name_and_args.push(eval_expr(arg, context, frame).await?);
  }

  eval_command_inner(&mut name_and_args, context, frame).await
}
