use crate::parser;
use crate::parser::{ParseError, WordNode};

pub enum ExprNode {
  Word(WordNode),
  UnaryOp(UnaryOp, Box<ExprNode>),
  BinaryOp(BinaryOp, Box<ExprNode>, Box<ExprNode>),
  Ternary(Box<ExprNode>, Box<ExprNode>, Box<ExprNode>),
}

pub enum UnaryOp {
  Plus,
  Minus,
}

pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
}

pub fn parse_expr(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_binary1(src)
}

pub fn parse_expr_binary1(mut src: &str) -> Result<(ExprNode, &str), ParseError> {
  let (left, rest) = parse_expr_binary2(src)?;
  src = rest;

  while !rest.is_empty() {
    if let Ok((_, rest)) = parser::parse_ws(rest) {
      src = rest;
    }

    if let Ok((op, rest)) = parse_binary_operator(src) {
    } else {
      break;
    }
  }
  Ok((left, src))
}

pub fn parse_expr_binary2(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_unary(src)
}

pub fn parse_binary_operator(src: &str) -> Result<(String, &str), ParseError> {
  Ok(("+".to_string(), src))
}

pub fn parse_expr_unary(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_atom(src)
}

pub fn parse_expr_atom(src: &str) -> Result<(ExprNode, &str), ParseError> {
  let (word, rest) = parser::parse_word(src)?;
  Ok((ExprNode::Word(word), rest))
}
