use regex::Regex;

use crate::parser;
use crate::parser::{ParseError, WordNode};

#[derive(Debug, PartialEq)]
pub enum ExprNode {
  Word(WordNode),
  UnaryOp(UnaryOp, Box<ExprNode>),
  BinaryOp(BinaryOp, Box<ExprNode>, Box<ExprNode>),
  Ternary(Box<ExprNode>, Box<ExprNode>, Box<ExprNode>),
}

#[derive(Debug, PartialEq)]
pub enum UnaryOp {
  Plus,
  Minus,
}

#[derive(Debug, PartialEq)]
pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
}

pub fn parse_expr(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_binary(src, 0)
}

pub fn parse_expr_binary(mut src: &str, precedence: u8) -> Result<(ExprNode, &str), ParseError> {
  if precedence >= 2 {
    return parse_expr_unary(src);
  }

  let (mut left, rest) = parse_expr_binary(src, precedence + 1)?;
  src = rest;

  while !src.is_empty() {
    if let Ok((_, rest)) = parser::parse_ws(src) {
      src = rest;
    }

    let Ok((op, rest)) = parse_binary_operator(src, precedence) else {
      break;
    };
    src = rest;

    if let Ok((_, rest)) = parser::parse_ws(src) {
      src = rest;
    }

    let (right, rest) = parse_expr_binary(src, precedence + 1)?;
    left = ExprNode::BinaryOp(op, Box::new(left), Box::new(right));
    src = rest;
  }

  Ok((left, src))
}

pub fn parse_binary_operator(src: &str, precedence: u8) -> Result<(BinaryOp, &str), ParseError> {
  let re = match precedence {
    0 => Regex::new("^[+-]").unwrap(),
    1 => Regex::new("^[*/]").unwrap(),
    _ => return Err(ParseError::Internal("invalid precedence".to_string())),
  };

  let Some(captures) = re.captures(src) else {
    return Err(ParseError::Generic("expected binary operator".to_string()));
  };
  let m = captures.get_match().as_str();

  let op = match m {
    "+" => BinaryOp::Add,
    "-" => BinaryOp::Sub,
    "*" => BinaryOp::Mul,
    "/" => BinaryOp::Div,
    _ => {
      return Err(ParseError::Internal(
        "regex did not match an operator".to_string(),
      ));
    }
  };

  Ok((op, &src[m.len()..]))
}

pub fn parse_expr_unary(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_atom(src)
}

pub fn parse_expr_atom(src: &str) -> Result<(ExprNode, &str), ParseError> {
  let (word, rest) = parser::parse_word(src)?;
  Ok((ExprNode::Word(word), rest))
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! lit {
    ($s: literal) => {
      ExprNode::Word(WordNode {
        parts: vec![parser::WordPart::BareLiteral($s.to_string())],
      })
    };
  }

  macro_rules! binop {
    ($op: ident, $left: expr, $right: expr) => {
      ExprNode::BinaryOp(BinaryOp::$op, Box::new($left), Box::new($right))
    };
  }

  #[test]
  fn parses_binary_op() -> Result<(), ParseError> {
    let (node, _) = parse_expr("1 + 2")?;
    assert_eq!(node, binop!(Add, lit!("1"), lit!("2")));
    Ok(())
  }

  #[test]
  fn parses_binary_ops_successsive() -> Result<(), ParseError> {
    let (node, _) = parse_expr("1 + 2 + 3")?;
    assert_eq!(
      node,
      binop!(Add, binop!(Add, lit!("1"), lit!("2")), lit!("3"))
    );
    Ok(())
  }

  #[test]
  fn parses_binary_ops_precedence() -> Result<(), ParseError> {
    let (node, _) = parse_expr("1 + 2 * 3")?;
    assert_eq!(
      node,
      binop!(Add, lit!("1"), binop!(Mul, lit!("2"), lit!("3")))
    );
    Ok(())
  }

  #[test]
  fn parses_binary_ops_big() -> Result<(), ParseError> {
    let (node, _) = parse_expr("2 + 3 * 2 + 2 * 3 * 5")?;
    assert_eq!(
      node,
      binop!(
        Add,
        binop!(Add, lit!("2"), binop!(Mul, lit!("3"), lit!("2"))),
        binop!(Mul, binop!(Mul, lit!("2"), lit!("3")), lit!("5"))
      )
    );
    Ok(())
  }
}
