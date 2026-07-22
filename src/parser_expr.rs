use regex::Regex;

use crate::parser::{self, ParseMode, WordPart};
use crate::parser::{ParseError, WordNode};

use std::mem::take;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq)]
pub enum ExprNode {
  Word(WordNode),
  UnaryOp(UnaryOp, Box<ExprNode>),
  BinaryOp(BinaryOp, Box<ExprNode>, Box<ExprNode>),
  Ternary(Box<ExprNode>, Box<ExprNode>, Box<ExprNode>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
  Plus,
  Minus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Eq,
  Ne,
  Lt,
  Gt,
  Le,
  Ge,
}

pub fn parse_expr(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_binary(src, 0)
}

pub fn parse_expr_binary(mut src: &str, precedence: u8) -> Result<(ExprNode, &str), ParseError> {
  if precedence >= 3 {
    return parse_expr_unary(src);
  }

  let (mut left, rest) = parse_expr_binary(src, precedence + 1)?;
  src = rest;

  while !src.is_empty() {
    match parser::parse_ws(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => {}
    }

    let (op, rest) = match parse_binary_operator(src, precedence) {
      Ok(result) => result,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => break,
    };
    src = rest;

    match parser::parse_ws(src) {
      Ok((_, rest)) => src = rest,
      Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
      Err(_) => {}
    }

    let (right, rest) = parse_expr_binary(src, precedence + 1)?;
    left = ExprNode::BinaryOp(op, Box::new(left), Box::new(right));
    src = rest;
  }

  Ok((left, src))
}

pub fn parse_binary_operator(src: &str, precedence: u8) -> Result<(BinaryOp, &str), ParseError> {
  static RE_P0: LazyLock<Regex> = LazyLock::new(|| Regex::new("^(<=|==|!=|>=|<|>)").unwrap());
  static RE_P1: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[+-]").unwrap());
  static RE_P2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[*/%]").unwrap());

  let re = match precedence {
    0 => &RE_P0,
    1 => &RE_P1,
    2 => &RE_P2,
    _ => return Err(ParseError::Internal("invalid precedence".to_string())),
  };

  let Some(captures) = re.captures(src) else {
    return Err(ParseError::Generic("expected binary operator".to_string()));
  };
  let m = captures.get_match().as_str();

  let op = match m {
    "<" => BinaryOp::Lt,
    "<=" => BinaryOp::Le,
    "==" => BinaryOp::Eq,
    "!=" => BinaryOp::Ne,
    ">=" => BinaryOp::Ge,
    ">" => BinaryOp::Gt,
    "+" => BinaryOp::Add,
    "-" => BinaryOp::Sub,
    "*" => BinaryOp::Mul,
    "/" => BinaryOp::Div,
    "%" => BinaryOp::Rem,
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
  match parse_expr_group(src) {
    Ok(result) => return Ok(result),
    Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
    Err(_) => {}
  }

  let (word, rest) = parse_expr_word(src)?;
  Ok((word, rest))
}

fn parse_expr_word(mut src: &str) -> Result<(ExprNode, &str), ParseError> {
  let mut part_buffer = String::new();
  let mut parts: Vec<WordPart> = vec![];

  loop {
    match src.chars().next() {
      None
      | Some(' ' | '\t' | '\r' | '\n')
      | Some(
        '(' | ')' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '&' | '|' | '?' | ':',
      ) => {
        if !part_buffer.is_empty() {
          parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
        }

        return Ok((ExprNode::Word(WordNode { parts }), src));
      }

      Some('$') => {
        if !part_buffer.is_empty() {
          parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
        }

        let (parsed, rest) = parser::parse_varsub(src)?;
        parts.push(parsed);
        src = rest;
      }

      Some('[') => {
        if !part_buffer.is_empty() {
          parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
        }

        let (parsed, rest) = parser::parse_cmdsub(src)?;
        parts.push(parsed);
        src = rest;
      }

      Some('"') => {
        if !part_buffer.is_empty() {
          parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
        }

        let (parsed, rest) = parser::parse_quoted(src, ParseMode::Script)?;
        parts.push(parsed);
        src = rest;
      }

      Some(ch) => {
        part_buffer.push(ch);
        src = &src[ch.len_utf8()..];
      }
    }
  }
}

pub fn parse_expr_group(src: &str) -> Result<(ExprNode, &str), ParseError> {
  let Some(src) = src.strip_prefix("(") else {
    return Err(ParseError::Generic("expected open parenthesis".to_string()));
  };

  let (node, src) = parse_expr(src)?;

  let Some(src) = src.strip_prefix(")") else {
    return Err(ParseError::Continuable(
      "missing closing character: )".to_string(),
    ));
  };

  Ok((node, src))
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
    let (node, _) = parse_expr("5 % 2")?;
    assert_eq!(node, binop!(Rem, lit!("5"), lit!("2")));
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

  #[test]
  fn parses_group_simple() -> Result<(), ParseError> {
    let (node, _) = parse_expr("(2 + 3)")?;
    assert_eq!(node, binop!(Add, lit!("2"), lit!("3")));
    Ok(())
  }

  #[test]
  fn parses_group_precedence_change() -> Result<(), ParseError> {
    let (node, _) = parse_expr("5 * (2 + 3)")?;
    assert_eq!(
      node,
      binop!(Mul, lit!("5"), binop!(Add, lit!("2"), lit!("3")))
    );
    Ok(())
  }

  #[test]
  fn parses_group_nested() -> Result<(), ParseError> {
    let (node, _) = parse_expr("5 * ((2 + 3) * 4)")?;
    assert_eq!(
      node,
      binop!(
        Mul,
        lit!("5"),
        binop!(Mul, binop!(Add, lit!("2"), lit!("3")), lit!("4"))
      )
    );
    Ok(())
  }

  #[test]
  fn parses_comparison_ops() -> Result<(), ParseError> {
    assert_eq!(parse_expr("1 < 2")?.0, binop!(Lt, lit!("1"), lit!("2")));
    assert_eq!(parse_expr("1 <= 2")?.0, binop!(Le, lit!("1"), lit!("2")));
    assert_eq!(parse_expr("1 == 2")?.0, binop!(Eq, lit!("1"), lit!("2")));
    assert_eq!(parse_expr("1 != 2")?.0, binop!(Ne, lit!("1"), lit!("2")));
    assert_eq!(parse_expr("1 >= 2")?.0, binop!(Ge, lit!("1"), lit!("2")));
    assert_eq!(parse_expr("1 > 2")?.0, binop!(Gt, lit!("1"), lit!("2")));
    Ok(())
  }

  #[test]
  fn skips_leading_and_trailing_whitespace() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("  1 + 2  ")?;
    assert_eq!(node, binop!(Add, lit!("1"), lit!("2")));
    assert!(rest.chars().all(|c| c.is_whitespace()));
    Ok(())
  }

  #[test]
  fn skips_whitespace_inside_groups() -> Result<(), ParseError> {
    let (node, _) = parse_expr("( 1 + 2 )")?;
    assert_eq!(node, binop!(Add, lit!("1"), lit!("2")));
    let (node, _) = parse_expr("1 + ( 2 * 3 )")?;
    assert_eq!(node, binop!(Add, lit!("1"), binop!(Mul, lit!("2"), lit!("3"))));
    Ok(())
  }

  #[test]
  fn parses_compact_binary_ops() -> Result<(), ParseError> {
    let (node, _) = parse_expr("1/2")?;
    assert_eq!(node, binop!(Div, lit!("1"), lit!("2")));
    let (node, _) = parse_expr("5%2")?;
    assert_eq!(node, binop!(Rem, lit!("5"), lit!("2")));
    Ok(())
  }

  #[test]
  fn parses_braced_literal_atoms() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("{2}")?;
    assert_eq!(
      node,
      ExprNode::Word(WordNode {
        parts: vec![parser::WordPart::BracedLiteral("2".to_string())],
      })
    );
    assert_eq!(rest, "");

    let (node, _) = parse_expr("{2} + 3")?;
    assert_eq!(
      node,
      binop!(
        Add,
        ExprNode::Word(WordNode {
          parts: vec![parser::WordPart::BracedLiteral("2".to_string())],
        }),
        lit!("3")
      )
    );
    Ok(())
  }

  #[test]
  fn rejects_operator_without_operands() {
    assert!(parse_expr("==").is_err());
    assert!(parse_expr("+").is_err());
    assert!(parse_expr("1 +").is_err());
  }

  #[test]
  fn leaves_unsupported_operators_unconsumed() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("0||1")?;
    assert_eq!(node, lit!("0"));
    assert_eq!(rest, "||1");
    Ok(())
  }
}
