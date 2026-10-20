use regex::Regex;

use crate::eval_error::EvalError;
use crate::parser;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops;
use std::rc::Rc;
use std::sync::LazyLock;

pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;
pub enum Number {
  Int(i64),
  Float(f64),
}

#[derive(Clone, Debug)]
pub enum Repr {
  None,
  Int(i64),
  Float(f64),
  Dict(Rc<Dict>),
  List(Rc<List>),
}

#[derive(Clone, Debug)]
pub struct Value {
  string: Option<Rc<str>>,
  repr: Repr,
}

/// Return "s" if it's a valid Tcl bare word, else return "{s}"
pub fn maybe_quote(s: &str) -> String {
  static RE_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[^$\[\]{}()";\s]+$"#).unwrap());

  if RE_WORD.is_match(s) {
    return String::from(s);
  }

  format!("{{{}}}", s)
}

impl Value {
  pub fn none() -> Value {
    Value {
      string: Some("".into()),
      repr: Repr::None,
    }
  }

  pub fn new(x: impl Into<Rc<str>>) -> Value {
    Value {
      string: Some(x.into()),
      repr: Repr::None,
    }
  }

  /// Converts the internal Repr into a string, or returns the existing string.
  ///
  /// Prefer repr_str(), which memoizes the conversion.
  pub fn format_string(&self) -> Result<Rc<str>, EvalError> {
    if let Some(string) = &self.string {
      return Ok(Rc::clone(string));
    }

    match &self.repr {
      Repr::Int(i) => Ok(i.to_string().into()),
      Repr::Float(f) => Ok(format_float_tcl(*f).into()),
      Repr::List(l) => {
        let mut result = String::new();
        for v in l.iter() {
          if !result.is_empty() {
            result.push(' ')
          }
          result.push_str(maybe_quote(v.to_string().as_str()).as_str());
        }
        Ok(result.into())
      }
      Repr::Dict(d) => {
        let mut result = String::new();
        for (k, v) in d.iter() {
          if !result.is_empty() {
            result.push(' ');
          }
          result.push_str(
            format!("{} {}", maybe_quote(k), maybe_quote(v.to_string().as_str())).as_str(),
          );
        }
        Ok(result.into())
      }
      Repr::None => Err(EvalError::Generic(
        "Internal error: no string and no repr".to_string(),
      )),
    }
  }

  /// Gets the string representation of the value, caching the result.
  pub fn repr_str(&mut self) -> Result<&str, EvalError> {
    if self.string.is_none() {
      self.string = Some(self.format_string()?);
    }

    Ok(self.string.as_deref().unwrap())
  }

  pub fn repr_bool(&mut self) -> Result<bool, EvalError> {
    match self.repr_number() {
      Ok(Number::Int(x)) => return Ok(x != 0),
      Ok(Number::Float(x)) => return Ok(x != 0.),
      Err(_) => {}
    }

    let s = self.repr_str()?;

    static RE_TRUTHY: LazyLock<Regex> =
      LazyLock::new(|| Regex::new("(?i)^(tr?u?e?|ye?s?|on)$").unwrap());
    if RE_TRUTHY.is_match(s) {
      return Ok(true);
    }

    static RE_FALSY: LazyLock<Regex> =
      LazyLock::new(|| Regex::new("(?i)^(fa?l?s?e?|no?|off?)$").unwrap());
    if RE_FALSY.is_match(s) {
      return Ok(false);
    }

    Err(EvalError::ArgumentError(format!(
      "expected boolean value but got \"{}\"",
      s
    )))
  }

  pub fn repr_int(&mut self) -> Result<i64, EvalError> {
    if let Repr::Int(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .repr_str()?
      .parse::<i64>()
      .map_err(|e| EvalError::Generic(format!("{}: {}", e, self)))?;
    self.repr = Repr::Int(x);
    Ok(x)
  }

  pub fn repr_float(&mut self) -> Result<f64, EvalError> {
    if let Repr::Float(x) = self.repr {
      return Ok(x);
    }
    if let Repr::Int(x) = self.repr {
      return Ok(x as f64);
    }

    let x: f64 = self
      .repr_str()?
      .parse::<f64>()
      .map_err(|e| EvalError::Generic(format!("{}: {}", e, self)))?;
    self.repr = Repr::Float(x);
    Ok(x)
  }

  pub fn repr_number(&mut self) -> Result<Number, EvalError> {
    match self.repr {
      Repr::Int(x) => return Ok(Number::Int(x)),
      Repr::Float(x) => return Ok(Number::Float(x)),
      Repr::None | Repr::Dict(_) | Repr::List(_) => {}
    }
    if let Ok(x) = self.repr_int() {
      return Ok(Number::Int(x));
    }
    if let Ok(x) = self.repr_float() {
      return Ok(Number::Float(x));
    }
    Err(EvalError::NotNumericError(self.to_string()))
  }

  /// may or may not return a number if this Value is a number, otherwise returns None.
  pub fn peek_number(&self) -> Option<Number> {
    match self.repr {
      Repr::Int(x) => Some(Number::Int(x)),
      Repr::Float(x) => Some(Number::Float(x)),
      Repr::None | Repr::Dict(_) | Repr::List(_) => None,
    }
  }

  pub fn repr_dict(&mut self) -> Result<Rc<Dict>, EvalError> {
    if let Repr::Dict(dict) = &self.repr {
      return Ok(Rc::clone(dict));
    }

    let str = self.repr_str()?;
    let (words, "") = parser::parse_list(str)
      .map_err(|e| EvalError::Generic(format!("failed to parse as dict: {}", e)))?
    else {
      return Err(EvalError::Generic(
        "failed to parse dict: extra input".to_string(),
      ));
    };

    let mut dict = Dict::new();
    let mut it = words.iter();
    loop {
      let Some(k) = it.next() else { break };
      let Some(v) = it.next() else {
        return Err(EvalError::Generic(format!(
          "invalid dict; missing value for key {}",
          k
        )));
      };

      dict.insert(k.to_string(), Value::from(v.as_str()));
    }

    self.repr = Repr::Dict(dict.into());
    let Repr::Dict(dict) = &self.repr else {
      unreachable!()
    };
    Ok(dict.clone())
  }

  pub fn repr_list(&mut self) -> Result<Rc<List>, EvalError> {
    if let Repr::List(list) = &self.repr {
      return Ok(Rc::clone(list));
    }

    let str = self.repr_str()?;
    let (words, "") = parser::parse_list(str)
      .map_err(|e| EvalError::Generic(format!("failed to parse as list: {}", e)))?
    else {
      return Err(EvalError::Generic(
        "failed to parse list: extra input".to_string(),
      ));
    };

    let mut list = List::new();
    for word in words {
      list.push(Value::new(word));
    }
    self.repr = Repr::List(list.into());
    let Repr::List(list) = &self.repr else {
      unreachable!()
    };
    Ok(list.clone())
  }

  pub fn unary_plus(&mut self) -> Result<Value, EvalError> {
    match self.repr_number() {
      Ok(Number::Int(x)) => Ok(Value::from(x)),
      Ok(Number::Float(x)) => Ok(Value::from(x)),
      Err(_) => Err(EvalError::ArgumentError(
        "can't use non-numeric string as operand of +".to_string(),
      )),
    }
  }

  pub fn bit_not(&mut self) -> Result<Value, EvalError> {
    if let Ok(x) = self.repr_int() {
      return Ok(Value::from(!x));
    }

    Err(EvalError::ArgumentError(
      "operator ~ expects integer operand".to_string(),
    ))
  }

  pub fn compare(&mut self, other: &mut Value) -> Result<Option<std::cmp::Ordering>, EvalError> {
    if let (Ok(a), Ok(b)) = (self.repr_number(), other.repr_number()) {
      return match (a, b) {
        (Number::Int(a), Number::Int(b)) => Ok(Some(a.cmp(&b))),
        _ => Ok(self.repr_float()?.partial_cmp(&other.repr_float()?)),
      };
    }

    return Ok(Some(self.repr_str()?.cmp(other.repr_str()?)));
  }

  pub fn lt(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if matches!(self.compare(other)?, Some(Ordering::Less)) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub fn le(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if matches!(
      self.compare(other)?,
      Some(Ordering::Less) | Some(Ordering::Equal)
    ) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub fn eq(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if matches!(self.compare(other)?, Some(Ordering::Equal)) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub fn ne(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if !matches!(self.compare(other)?, Some(Ordering::Equal)) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub fn ge(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if matches!(
      self.compare(other)?,
      Some(Ordering::Greater) | Some(Ordering::Equal)
    ) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub fn gt(&mut self, other: &mut Value) -> Result<Value, EvalError> {
    if matches!(self.compare(other)?, Some(Ordering::Greater)) {
      Ok(Value::from(1))
    } else {
      Ok(Value::from(0))
    }
  }

  pub(crate) fn is_nan(&self) -> bool {
    match self.repr {
      Repr::Float(x) => x.is_nan(),
      Repr::None | Repr::Int(_) | Repr::Dict(_) | Repr::List(_) => false,
    }
  }

  pub fn is_numeric(&mut self) -> bool {
    match self.repr_number() {
      Ok(Number::Int(_)) => true,
      Ok(Number::Float(x)) => !x.is_nan(),
      Err(_) => false,
    }
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.format_string() {
      Ok(s) => f.write_str(&s),
      Err(_) => f.write_str("{Unprintable Value}"),
    }
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Value {
      string: Some(value.into()),
      repr: Repr::None,
    }
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value {
      string: Some(value.into()),
      repr: Repr::None,
    }
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Value {
      string: None,
      repr: Repr::Int(if value { 1 } else { 0 }),
    }
  }
}

impl From<i64> for Value {
  fn from(value: i64) -> Self {
    Value {
      string: None,
      repr: Repr::Int(value),
    }
  }
}

impl From<f64> for Value {
  fn from(value: f64) -> Self {
    Value {
      string: None,
      repr: Repr::Float(value),
    }
  }
}

impl From<Dict> for Value {
  fn from(value: Dict) -> Self {
    Value {
      string: None,
      repr: Repr::Dict(Rc::new(value)),
    }
  }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
  fn from(value: Vec<T>) -> Self {
    Value {
      string: None,
      repr: Repr::List(Rc::new(value.into_iter().map(Into::into).collect())),
    }
  }
}

macro_rules! arithmetic_binop {
  ($trait: ident, $name: ident, $op: tt) => {
    impl ops::$trait for Value {
      type Output = Result<Value, EvalError>;

      fn $name(mut self, mut rhs: Self) -> Self::Output {
        let a = self.repr_number()?;
        let b = rhs.repr_number()?;

        match (a, b) {
          (Number::Int(a), Number::Int(b)) => Ok(Value::from(a $op b)),
          _ => Ok(Value::from(self.repr_float()? $op rhs.repr_float()?)),
        }
      }
    }
  };
}

arithmetic_binop!(Add, add, +);
arithmetic_binop!(Sub, sub, -);
arithmetic_binop!(Mul, mul, *);
arithmetic_binop!(Div, div, /);

impl ops::Rem for Value {
  type Output = Result<Value, EvalError>;

  fn rem(mut self, mut rhs: Self) -> Self::Output {
    let a = self.repr_int()?;
    let b = rhs.repr_int()?;
    return Ok(Value::from(a % b));
  }
}

impl ops::Neg for Value {
  type Output = Result<Value, EvalError>;

  fn neg(mut self) -> Self::Output {
    match self.repr_number() {
      Ok(Number::Int(x)) => Ok(Value::from(-x)),
      Ok(Number::Float(x)) => Ok(Value::from(-x)),
      Err(_) => Err(EvalError::ArgumentError(format!(
        "can't use non-numeric string as operand of -"
      ))),
    }
  }
}

impl ops::Not for Value {
  type Output = Result<Value, EvalError>;

  fn not(mut self) -> Self::Output {
    if let Ok(b) = self.repr_bool() {
      return Ok(Value::from(!b));
    }
    return Err(EvalError::ArgumentError(format!(
      "can't use non-numeric string as operand of !"
    )));
  }
}

fn format_float_tcl(x: f64) -> String {
  if x.is_nan() {
    return "NaN".into();
  }
  if x.is_infinite() {
    if x.is_sign_positive() {
      return "Inf".into();
    } else {
      return "-Inf".into();
    }
  }

  let sci = format!("{x:e}");
  let (_, exponent) = sci.split_once('e').unwrap();
  match exponent.parse::<i32>().unwrap() {
    ..-4 => sci,
    17.. => sci.replace("e", "e+"),
    _ => {
      let fixed = x.to_string();
      if fixed.contains('.') {
        fixed
      } else {
        fixed + ".0"
      }
    }
  }
}
