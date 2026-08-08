use super::{EvalContext, FrameId, eval_word};
use crate::eval_error::EvalError;
use crate::parser_expr::{BinaryOp, ExprNode};
use crate::value::Value;

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
    Rem => a % b,
  }
}
