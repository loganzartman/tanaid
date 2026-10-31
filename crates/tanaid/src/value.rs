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
/// A Value can be constructed from any type representable by ValueData.
/// This avoids converting the Value to and from a String until requested.
///
/// A Value may be reinterpreted as another type of Repr. For example, "2" may be interpreted as a string, an integer, a float, or a list containing the single element "2".
/// Reinterpreting is done by calling the corresponding `repr_` method.
/// - Reinterpreting can fail; for example, "2.2" cannot be interpreted as an integer. This returns an EvalError.
/// - When a Value is reinterpreted successfully, the resulting Rust value is cached within the Value for future repr_ calls.
/// - Unlike in tclsh, all cached reprs are retained for the lifetime of the Value.
#[derive(Clone, Debug)]
pub struct Value {
  /// Primary, authoritative data for this Value. Source for all other reprs.
  data: ValueData,
  /// Lazily-cached converted forms of the Value's data.
  // OnceCell'd to avoid heap alloc for values that are never reinterpreted
  reprs: OnceCell<Rc<CachedReprs>>,
}

/// The primary, authoritative data for a Value.
#[derive(Clone, Debug)]
pub enum ValueData {
  Number(Number),
  String(Rc<str>),
  List(Rc<List>),
  Dict(Rc<Dict>),
}

/// Lazily-cached converted forms of the Value's data.
#[derive(Clone, Debug)]
struct CachedReprs {
  pub string: OnceCell<String>,
  pub number: OnceCell<Option<Number>>,
  pub dict: OnceCell<Option<Dict>>,
  pub list: OnceCell<Option<List>>,
}

impl CachedReprs {
  pub fn new() -> Self {
    Self {
      string: OnceCell::new(),
      number: OnceCell::new(),
      dict: OnceCell::new(),
      list: OnceCell::new(),
    }
  }
}

#[derive(Clone, Debug)]
pub enum Number {
  Int(Int),
  Float(Float),
}

impl Value {
  pub fn none() -> Value {
    Value {
      data: ValueData::String(Rc::from("")),
      reprs: OnceCell::new(),
    }
  }

  pub fn new(data: ValueData) -> Value {
    Value {
      data,
      reprs: OnceCell::new(),
    }
  }

  pub fn new_literal(_str: &str) -> Value {
    todo!()
  }

  /// Converts the source into a string.
  fn format_string(&self) -> String {
    match &self.data {
      ValueData::String(s) => s.to_string(),
      ValueData::Number(n) => match n {
        Number::Int(i) => i.to_string().into(),
        Number::Float(f) => format_float_tcl(*f).into(),
      },
      ValueData::List(l) => {
        let mut result = String::new();
        for v in l.iter() {
          if !result.is_empty() {
            result.push(' ')
          }
          result.push_str(maybe_quote(v.to_string().as_str()).as_str());
        }
        result.into()
      }
      ValueData::Dict(d) => {
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

  fn reprs(&self) -> &Rc<CachedReprs> {
    self.reprs.get_or_init(|| Rc::new(CachedReprs::new()))
  }

  /// Gets the string representation of the value, caching the result.
  pub fn repr_str(&self) -> &str {
    if let ValueData::String(s) = &self.data {
      return s;
    }

    self
      .reprs()
      .string
      .get_or_init(|| self.format_string())
      .as_str()
  }

  pub fn repr_bool(&self) -> Result<bool, EvalError> {
    match self.repr_number() {
      Ok(Number::Int(x)) => return Ok(*x != 0),
      Ok(Number::Float(x)) => return Ok(*x != 0.),
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
    if let ValueData::Number(n) = &self.data {
      return match n {
        Number::Int(x) => Ok(*x),
        Number::Float(x) => Err(EvalError::Generic(format!("not a valid int: {x}"))),
      };
    }

    match self.repr_number()? {
      Number::Int(x) => Ok(*x),
      Number::Float(x) => Err(EvalError::Generic(format!("not a valid int: {x}"))),
    }
  }

  pub fn repr_float(&self) -> Result<f64, EvalError> {
    if let ValueData::Number(n) = &self.data {
      return match n {
        Number::Int(x) => Ok(*x as f64),
        Number::Float(x) => Ok(*x),
      };
    }

    match self.repr_number()? {
      Number::Int(x) => Ok(*x as f64),
      Number::Float(x) => Ok(*x),
    }
  }

  pub fn repr_number(&self) -> Result<&Number, EvalError> {
    if let ValueData::Number(n) = &self.data {
      return Ok(n);
    }

    self
      .reprs()
      .number
      .get_or_init(|| {
        if let Ok(x) = self.repr_str().parse::<Int>() {
          return Some(Number::Int(x));
        }
        if let Ok(x) = self.repr_str().parse::<Float>() {
          return Some(Number::Float(x));
        }
        None
      })
      .as_ref()
      .ok_or_else(|| EvalError::NotNumericError(self.to_string()))
  }

  pub fn repr_dict(&self) -> Result<&Dict, EvalError> {
    if let ValueData::Dict(dict) = &self.data {
      return Ok(dict.as_ref());
    }

    self
      .reprs()
      .dict
      .get_or_init(|| {
        let str = self.repr_str();
        let Ok((words, "")) = parser::parse_list(str) else {
          return None;
        };

        let mut dict = Dict::new();
        let mut it = words.iter();
        loop {
          let Some(k) = it.next() else { break };
          let Some(v) = it.next() else { return None };

          dict.insert(k.to_string(), Value::from(v.as_str()));
        }

        Some(dict)
      })
      .as_ref()
      .ok_or_else(|| EvalError::Generic(format!("value is not a valid dict: {}", self.repr_str())))
  }

  pub fn repr_list(&self) -> Result<&List, EvalError> {
    if let ValueData::List(list) = &self.data {
      return Ok(list.as_ref());
    }

    self
      .reprs()
      .list
      .get_or_init(|| {
        let str = self.repr_str();
        let Ok((words, "")) = parser::parse_list(str) else {
          return None;
        };

        let mut list = List::new();
        for word in words {
          list.push(Value::from(word));
        }
        Some(list)
      })
      .as_ref()
      .ok_or_else(|| EvalError::Generic(format!("value is not a valid list: {}", self.repr_str())))
  }

  pub fn unary_plus(&mut self) -> Result<Value, EvalError> {
    match self.repr_number() {
      Ok(Number::Int(x)) => Ok(Value::from(*x)),
      Ok(Number::Float(x)) => Ok(Value::from(*x)),
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

  /// return whether this value has a cached numeric repr
  pub fn is_already_number(&self) -> bool {
    match &self.data {
      ValueData::Number(_) => true,
      _ => matches!(self.reprs.get().and_then(|r| r.number.get()), Some(Some(_))),
    }
  }

  /// return whether this value has a cached NaN repr
  pub(crate) fn is_already_nan(&self) -> bool {
    match &self.data {
      ValueData::Number(x) => match x {
        Number::Float(x) => return x.is_nan(),
        Number::Int(_) => false,
      },
      ValueData::String(_) | ValueData::Dict(_) | ValueData::List(_) => false,
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
    Value::new(ValueData::String(Rc::from(value)))
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value::new(ValueData::String(Rc::from(value)))
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Value::new(ValueData::Number(Number::Int(if value { 1 } else { 0 })))
  }
}

impl From<Int> for Value {
  fn from(value: Int) -> Self {
    Value::new(ValueData::Number(Number::Int(value)))
  }
}

impl From<Float> for Value {
  fn from(value: Float) -> Self {
    Value::new(ValueData::Number(Number::Float(value)))
  }
}

impl From<Dict> for Value {
  fn from(value: Dict) -> Self {
    Value::new(ValueData::Dict(Rc::new(value)))
  }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
  fn from(value: Vec<T>) -> Self {
    Value::new(ValueData::List(Rc::new(
      value.into_iter().map(Into::into).collect(),
    )))
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
