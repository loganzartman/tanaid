use regex::Regex;

use crate::parser::{self, WordPart};
use crate::parser::{ParseError, WordNode};

use std::mem::take;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq)]
pub enum ExprNode {
  Word(WordNode),
  UnaryOp(UnaryOp, Box<ExprNode>),
  BinaryOp(BinaryOp, Box<ExprNode>, Box<ExprNode>),
  Ternary(Box<ExprNode>, Box<ExprNode>, Box<ExprNode>),
  FnCall(String, Vec<ExprNode>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
  Plus,
  Minus,
  BitwiseNot,
  LogicalNot,
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
  And,
  Or,
}

pub fn parse_expr(src: &str) -> Result<(ExprNode, &str), ParseError> {
  parse_expr_binary(src, 0)
}

pub fn parse_expr_binary(mut src: &str, precedence: u8) -> Result<(ExprNode, &str), ParseError> {
  if precedence >= 5 {
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
  static RE_P0: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|\|").unwrap());
  static RE_P1: LazyLock<Regex> = LazyLock::new(|| Regex::new("^&&").unwrap());
  static RE_P2: LazyLock<Regex> = LazyLock::new(|| Regex::new("^(<=|==|!=|>=|<|>)").unwrap());
  static RE_P3: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[+-]").unwrap());
  static RE_P4: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[*/%]").unwrap());

  let re = match precedence {
    0 => &RE_P0,
    1 => &RE_P1,
    2 => &RE_P2,
    3 => &RE_P3,
    4 => &RE_P4,
    _ => return Err(ParseError::Internal("invalid precedence".to_string())),
  };

  let Some(captures) = re.captures(src) else {
    return Err(ParseError::Generic("expected binary operator".to_string()));
  };
  let m = captures.get_match().as_str();

  let op = match m {
    "||" => BinaryOp::Or,
    "&&" => BinaryOp::And,
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

pub fn parse_expr_unary(mut src: &str) -> Result<(ExprNode, &str), ParseError> {
  static RE_UNARY: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[+~!-]").unwrap());

  if let Ok((_, rest)) = parser::parse_ws(src) {
    src = rest;
  }

  if let Some(captures) = RE_UNARY.captures(src) {
    let m = captures.get_match().as_str();
    let op = match m {
      "+" => UnaryOp::Plus,
      "-" => UnaryOp::Minus,
      "~" => UnaryOp::BitwiseNot,
      "!" => UnaryOp::LogicalNot,
      _ => {
        return Err(ParseError::Internal(
          "regex did not match an operator".to_string(),
        ));
      }
    };
    src = &src[m.len()..];
    let (operand, rest) = parse_expr_unary(src)?;
    return Ok((ExprNode::UnaryOp(op, Box::new(operand)), rest));
  }

  parse_expr_atom(src)
}

pub fn parse_expr_atom(src: &str) -> Result<(ExprNode, &str), ParseError> {
  match parse_expr_group(src) {
    Ok(result) => return Ok(result),
    Err(err @ (ParseError::Continuable(_) | ParseError::Internal(_))) => return Err(err),
    Err(_) => {}
  }

  let (word, src) = parse_expr_word(src)?;

  {
    let mut src = src;
    if let Ok((_, rest)) = parser::parse_ws(src) {
      src = rest;
    }
    if src.starts_with('(') {
      return parse_expr_fncall(&word, src);
    }
  }

  Ok((word, src))
}

fn parse_expr_word(mut src: &str) -> Result<(ExprNode, &str), ParseError> {
  let mut part_buffer = String::new();
  let mut parts: Vec<WordPart> = vec![];

  match parser::parse_ws(src) {
    Ok((_, rest)) => src = rest,
    Err(_) => {}
  }

  let ch_first = src.chars().next();
  match ch_first {
    None => return Err(ParseError::Generic("expected word".to_string())),
    Some('{') => {
      let (parsed, rest) = parser::parse_braced_string(src)?;

      return Ok((
        ExprNode::Word(WordNode {
          parts: vec![WordPart::BracedLiteral(parsed)],
        }),
        rest,
      ));
    }
    Some('"') => {
      let (parsed, rest) = parser::parse_quoted(src)?;

      return Ok((
        ExprNode::Word(WordNode {
          parts: vec![parsed],
        }),
        rest,
      ));
    }
    _ => {}
  }

  loop {
    match src.chars().next() {
      None
      | Some(' ' | '\t' | '\r' | '\n')
      | Some(
        '(' | ')' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '&' | '|' | '?' | ':'
        | ',',
      ) => {
        if !part_buffer.is_empty() {
          parts.push(WordPart::BareLiteral(take(&mut part_buffer)));
        }

        if parts.is_empty() {
          return Err(ParseError::Generic("expected word".to_string()));
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

      Some(ch) => {
        part_buffer.push(ch);
        src = &src[ch.len_utf8()..];
      }
    }
  }
}

pub fn parse_expr_group(mut src: &str) -> Result<(ExprNode, &str), ParseError> {
  match parser::parse_ws(src) {
    Ok((_, rest)) => src = rest,
    Err(_) => {}
  }

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

pub fn parse_expr_fncall<'a>(
  expr_node: &ExprNode,
  mut src: &'a str,
) -> Result<(ExprNode, &'a str), ParseError> {
  if let Ok((_, rest)) = parser::parse_ws(src) {
    src = rest;
  }

  let Some(src) = src.strip_prefix('(') else {
    return Err(ParseError::Generic(
      "expected open paren for function call".to_string(),
    ));
  };

  let word = match expr_node {
    ExprNode::Word(word) => word,
    _ => return Err(ParseError::Generic("expected a word".to_string())),
  };

  let [part] = word.parts.as_slice() else {
    return Err(ParseError::Generic(
      "expected a word with one part".to_string(),
    ));
  };

  let func = match part {
    WordPart::BareLiteral(func) => func,
    _ => {
      return Err(ParseError::Generic(
        "expected a bare literal word".to_string(),
      ));
    }
  };

  let (args, mut src) = parse_expr_arglist(src)?;

  if let Ok((_, rest)) = parser::parse_ws(src) {
    src = rest;
  }

  let Some(src) = src.strip_prefix(')') else {
    return Err(ParseError::Continuable(format!(
      "expected close paren for function call, got: {}",
      src
    )));
  };

  Ok((ExprNode::FnCall(func.to_string(), args), src))
}

pub fn parse_expr_arglist(mut src: &str) -> Result<(Vec<ExprNode>, &str), ParseError> {
  let mut args = vec![];

  let mut first = true;
  loop {
    if let Ok((_, rest)) = parser::parse_ws(src) {
      src = rest;
    }

    if src.starts_with(')') {
      break;
    }

    // require comma after first arg
    if !first {
      let Some(rest) = src.strip_prefix(',') else {
        return Err(ParseError::Generic("expected , after argument".to_string()));
      };
      src = rest;
    } else {
      first = false;
    }

    let arg = match parse_expr(src) {
      Ok((word, rest)) => {
        src = rest;
        word
      }
      Err(err) => return Err(err),
    };
    args.push(arg);
  }

  Ok((args, src))
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

  macro_rules! unop {
    ($op: ident, $operand: expr) => {
      ExprNode::UnaryOp(UnaryOp::$op, Box::new($operand))
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
  fn parses_unary_ops() -> Result<(), ParseError> {
    for (src, expected) in [
      ("+1", unop!(Plus, lit!("1"))),
      ("-1", unop!(Minus, lit!("1"))),
      ("~1", unop!(BitwiseNot, lit!("1"))),
      ("!1", unop!(LogicalNot, lit!("1"))),
    ] {
      assert_eq!(parse_expr(src)?, (expected, ""));
    }
    Ok(())
  }

  #[test]
  fn parses_successive_unary_ops() -> Result<(), ParseError> {
    assert_eq!(
      parse_expr("! ~ -1")?,
      (
        unop!(LogicalNot, unop!(BitwiseNot, unop!(Minus, lit!("1")))),
        ""
      )
    );
    Ok(())
  }

  #[test]
  fn parses_unary_ops_precedence() -> Result<(), ParseError> {
    assert_eq!(
      parse_expr("-1 * 2 + 3")?.0,
      binop!(
        Add,
        binop!(Mul, unop!(Minus, lit!("1")), lit!("2")),
        lit!("3")
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
  fn parses_boolean_ops() -> Result<(), ParseError> {
    assert_eq!(parse_expr("0 && 1")?.0, binop!(And, lit!("0"), lit!("1")));
    assert_eq!(parse_expr("0 || 1")?.0, binop!(Or, lit!("0"), lit!("1")));
    assert_eq!(parse_expr("0&&1")?.0, binop!(And, lit!("0"), lit!("1")));
    assert_eq!(parse_expr("0||1")?.0, binop!(Or, lit!("0"), lit!("1")));
    Ok(())
  }

  #[test]
  fn parses_boolean_ops_successive() -> Result<(), ParseError> {
    assert_eq!(
      parse_expr("1 && 0 && 1")?.0,
      binop!(And, binop!(And, lit!("1"), lit!("0")), lit!("1"))
    );
    assert_eq!(
      parse_expr("0 || 1 || 0")?.0,
      binop!(Or, binop!(Or, lit!("0"), lit!("1")), lit!("0"))
    );
    Ok(())
  }

  #[test]
  fn parses_boolean_ops_precedence() -> Result<(), ParseError> {
    assert_eq!(
      parse_expr("1 || 0 && 0")?.0,
      binop!(Or, lit!("1"), binop!(And, lit!("0"), lit!("0")))
    );
    assert_eq!(
      parse_expr("1 == 2 || 3 < 4 && 5 != 6")?.0,
      binop!(
        Or,
        binop!(Eq, lit!("1"), lit!("2")),
        binop!(
          And,
          binop!(Lt, lit!("3"), lit!("4")),
          binop!(Ne, lit!("5"), lit!("6"))
        )
      )
    );
    assert_eq!(
      parse_expr("!0 && 1")?.0,
      binop!(And, unop!(LogicalNot, lit!("0")), lit!("1"))
    );
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
    assert_eq!(
      node,
      binop!(Add, lit!("1"), binop!(Mul, lit!("2"), lit!("3")))
    );
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
  fn rejects_empty_expr_words() {
    for src in ["=", "!", "?", ":", "&", "|", "()", "( )"] {
      assert!(
        parse_expr(src).is_err(),
        "expected error for empty expr word {src:?}"
      );
    }
  }

  #[test]
  fn leaves_unsupported_operators_unconsumed() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("0&1")?;
    assert_eq!(node, lit!("0"));
    assert_eq!(rest, "&1");
    Ok(())
  }

  #[test]
  fn parses_fncall() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("fn(arg)")?;
    assert_eq!(node, ExprNode::FnCall("fn".to_string(), vec![lit!("arg")]));
    assert_eq!(rest, "");
    Ok(())
  }

  #[test]
  fn parses_fncall_no_args() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("fn()")?;
    assert_eq!(node, ExprNode::FnCall("fn".to_string(), vec![]));
    assert_eq!(rest, "");
    Ok(())
  }

  #[test]
  fn parses_fncall_nested() -> Result<(), ParseError> {
    let (node, rest) = parse_expr("fn(fn(arg))")?;
    assert_eq!(
      node,
      ExprNode::FnCall(
        "fn".to_string(),
        vec![ExprNode::FnCall("fn".to_string(), vec![lit!("arg")])]
      )
    );
    assert_eq!(rest, "");
    Ok(())
  }
}
