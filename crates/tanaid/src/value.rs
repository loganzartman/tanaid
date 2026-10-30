use regex::Regex;

use crate::eval_error::EvalError;
use crate::parser;
use std::cell::OnceCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops;
use std::rc::Rc;
use std::sync::LazyLock;

pub type Int = i64;
pub type Float = f64;
pub type List = Vec<Value>;
pub type Dict = HashMap<String, Value>;

/// Immutable Tcl value.
///
/// Any Value is representable as a String. No information is lost when converting from a Value to a String and back.
///
/// A Value can be constructed from any type representable by Repr. This is the Value's `source`.
/// This avoids converting the Value to and from a String until requested.
///
/// A Value may be reinterpreted as another type of Repr. For example, "2" may be interpreted as a string, an integer, a float, or a list containing the single element "2".
/// Reinterpreting is done by calling the corresponding `repr_` method.
/// - Reinterpreting can fail; for example, "2.2" cannot be interpreted as an integer. This returns an EvalError.
/// - When a Value is reinterpreted successfully, the resulting Rust value is cached within the Value for future repr_ calls.
/// - Unlike in tclsh, all cached reprs are retained for the lifetime of the Value.
#[derive(Clone, Debug)]
pub struct Value(Rc<ValueData>);

#[derive(Clone, Debug)]
pub struct ValueData {
  source: Repr,
  repr_str: OnceCell<String>,
  repr_number: OnceCell<Result<Number, EvalError>>,
  repr_dict: OnceCell<Result<Dict, EvalError>>,
  repr_list: OnceCell<Result<List, EvalError>>,
}

/// A typed representation of a Tcl value.
#[derive(Clone, Debug)]
pub enum Repr {
  String(String),
  Int(Int),
  Float(Float),
  Dict(Dict),
  List(List),
}

#[derive(Clone, Debug)]
pub enum Number {
  Int(Int),
  Float(Float),
}

impl Default for ValueData {
  fn default() -> Self {
    Self {
      source: Repr::String("".into()),
      repr_str: Default::default(),
      repr_number: Default::default(),
      repr_dict: Default::default(),
      repr_list: Default::default(),
    }
  }
}

impl Value {
  pub fn none() -> Value {
    Value::new(Repr::String("".into()))
  }

  pub fn new(repr: Repr) -> Value {
    Value(Rc::new(ValueData {
      source: repr,
      ..Default::default()
    }))
  }

  /// Converts the source into a string.
  fn format_string(&self) -> String {
    match &self.0.source {
      Repr::String(s) => s.clone(),
      Repr::Int(i) => i.to_string().into(),
      Repr::Float(f) => format_float_tcl(*f).into(),
      Repr::List(l) => {
        let mut result = String::new();
        for v in l.iter() {
          if !result.is_empty() {
            result.push(' ')
          }
          result.push_str(maybe_quote(v.to_string().as_str()).as_str());
        }
        result.into()
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
        result.into()
      }
    }
  }

  /// Gets the string representation of the value, caching the result.
  pub fn repr_str(&self) -> &str {
    if let Repr::String(s) = &self.0.source {
      return s;
    }
    self.0.repr_str.get_or_init(|| self.format_string())
  }

  pub fn repr_bool(&self) -> Result<bool, EvalError> {
    match self.repr_number() {
      Ok(Number::Int(x)) => return Ok(x != 0),
      Ok(Number::Float(x)) => return Ok(x != 0.),
      Err(_) => {}
    }

    let s = self.repr_str();

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

  pub fn repr_int(&self) -> Result<Int, EvalError> {
    if let Repr::Int(x) = self.0.source {
      return Ok(x);
    }

    match self.repr_number()? {
      Number::Int(x) => Ok(x),
      Number::Float(x) => Err(EvalError::Generic(format!("not a valid int: {x}"))),
    }
  }

  pub fn repr_float(&self) -> Result<f64, EvalError> {
    if let Repr::Float(x) = self.0.source {
      return Ok(x);
    }
    if let Repr::Int(x) = self.0.source {
      return Ok(x as f64);
    }

    match self.repr_number()? {
      Number::Int(x) => Ok(x as f64),
      Number::Float(x) => Ok(x),
    }
  }

  pub fn repr_number(&self) -> Result<Number, EvalError> {
    cached(&self.0.repr_number, || {
      if let Ok(x) = self.repr_int() {
        return Ok(Number::Int(x));
      }
      if let Ok(x) = self.repr_float() {
        return Ok(Number::Float(x));
      }
      Err(EvalError::NotNumericError(self.to_string()))
    })
    .cloned()
  }

  /// return a number if this value has a cached numeric repr, else return None
  pub fn peek_number(&self) -> Option<Number> {
    match self.0.source {
      Repr::Int(i) => Some(Number::Int(i)),
      Repr::Float(f) => Some(Number::Float(f)),
      _ => self.0.repr_number.get()?.as_ref().ok().cloned(),
    }
  }

  pub fn repr_dict(&self) -> Result<&Dict, EvalError> {
    if let Repr::Dict(dict) = &self.0.source {
      return Ok(dict);
    }

    cached(&self.0.repr_dict, || {
      let str = self.repr_str();
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

      Ok(dict)
    })
  }

  pub fn repr_list(&self) -> Result<&List, EvalError> {
    if let Repr::List(list) = &self.0.source {
      return Ok(list);
    }

    cached(&self.0.repr_list, || {
      let str = self.repr_str();
      let (words, "") = parser::parse_list(str)
        .map_err(|e| EvalError::Generic(format!("failed to parse as list: {}", e)))?
      else {
        return Err(EvalError::Generic(
          "failed to parse list: extra input".to_string(),
        ));
      };

      let mut list = List::new();
      for word in words {
        list.push(Value::from(word));
      }
      Ok(list)
    })
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

    return Ok(Some(self.repr_str().cmp(other.repr_str())));
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
    match self.0.source {
      Repr::Float(x) => x.is_nan(),
      Repr::String(_) | Repr::Int(_) | Repr::Dict(_) | Repr::List(_) => false,
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
    f.write_str(self.repr_str())
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Value::new(Repr::String(value))
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value::new(Repr::String(value.into()))
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Value::new(Repr::Int(if value { 1 } else { 0 }))
  }
}

impl From<Int> for Value {
  fn from(value: Int) -> Self {
    Value::new(Repr::Int(value))
  }
}

impl From<Float> for Value {
  fn from(value: Float) -> Self {
    Value::new(Repr::Float(value))
  }
}

impl From<Dict> for Value {
  fn from(value: Dict) -> Self {
    Value::new(Repr::Dict(value))
  }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
  fn from(value: Vec<T>) -> Self {
    Value::new(Repr::List(value.into_iter().map(Into::into).collect()))
  }
}

macro_rules! arithmetic_binop {
  ($trait: ident, $name: ident, $op: tt) => {
    impl ops::$trait for Value {
      type Output = Result<Value, EvalError>;

      fn $name(self, rhs: Self) -> Self::Output {
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

  fn rem(self, rhs: Self) -> Self::Output {
    let a = self.repr_int()?;
    let b = rhs.repr_int()?;
    return Ok(Value::from(a % b));
  }
}

impl ops::Neg for Value {
  type Output = Result<Value, EvalError>;

  fn neg(self) -> Self::Output {
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

  fn not(self) -> Self::Output {
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

/// Return "s" if it's a valid Tcl bare word, else return "{s}"
pub fn maybe_quote(s: &str) -> String {
  static RE_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^[^$\[\]{}()";\s]+$"#).unwrap());

  if RE_WORD.is_match(s) {
    return String::from(s);
  }

  format!("{{{}}}", s)
}

fn cached<T>(
  cell: &OnceCell<Result<T, EvalError>>,
  f: impl FnOnce() -> Result<T, EvalError>,
) -> Result<&T, EvalError> {
  cell.get_or_init(f).as_ref().map_err(Clone::clone)
}
