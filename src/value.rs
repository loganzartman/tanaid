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
fn maybe_quote(s: &str) -> String {
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
      Repr::Float(f) => Ok(f.to_string().into()),
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

  pub fn repr_int(&mut self) -> Result<i64, EvalError> {
    if let Repr::Int(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .repr_str()?
      .parse::<i64>()
      .map_err(|e| EvalError::Generic(e.to_string()))?;
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
      .map_err(|e| EvalError::Generic(e.to_string()))?;
    self.repr = Repr::Float(x);
    Ok(x)
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
      .map_err(|e| EvalError::Generic(format!("failed to parse as dict: {}", e)))?
    else {
      return Err(EvalError::Generic(
        "failed to parse dict: extra input".to_string(),
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

  pub fn compare(&mut self, other: &mut Value) -> Result<Option<std::cmp::Ordering>, EvalError> {
    if let (Ok(a), Ok(b)) = (self.repr_int(), other.repr_int()) {
      return Ok(Some(a.cmp(&b)));
    }
    if let (Ok(a), Ok(b)) = (self.repr_float(), other.repr_float()) {
      return Ok(a.partial_cmp(&b));
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

impl From<List> for Value {
  fn from(value: List) -> Self {
    Value {
      string: None,
      repr: Repr::List(Rc::new(value)),
    }
  }
}

macro_rules! arithmetic_binop {
  ($trait: ident, $name: ident, $op: tt) => {
    impl ops::$trait for Value {
      type Output = Result<Value, EvalError>;

      fn $name(mut self, mut rhs: Self) -> Self::Output {
        if let (Ok(a), Ok(b)) = (self.repr_int(), rhs.repr_int()) {
          return Ok(Value::from(a $op b));
        }

        let Ok(a) = self.repr_float() else {
          return Err(EvalError::NotNumericError(self.to_string()));
        };
        let Ok(b) = rhs.repr_float() else {
          return Err(EvalError::NotNumericError(rhs.to_string()));
        };
        Ok(Value::from(a $op b))
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
