use crate::eval_error::EvalError;
use std::cmp::Ordering;
use std::fmt::Display;
use std::ops;

#[derive(Clone, Debug)]
pub enum Repr {
  None,
  Int(i64),
  Float(f64),
}

#[derive(Clone, Debug)]
pub struct Value {
  string: Option<String>,
  repr: Repr,
}

impl Value {
  pub fn none() -> Value {
    Value {
      string: Some("".to_string()),
      repr: Repr::None,
    }
  }

  pub fn new(x: impl Into<String>) -> Value {
    Value {
      string: Some(x.into()),
      repr: Repr::None,
    }
  }

  pub fn format_string(&self) -> Result<String, EvalError> {
    if let Some(string) = &self.string {
      return Ok(string.clone());
    }

    match self.repr {
      Repr::Int(i) => Ok(i.to_string()),
      Repr::Float(f) => Ok(f.to_string()),
      Repr::None => Err(EvalError::Generic(
        "Internal error: no string and no repr".to_string(),
      )),
    }
  }

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
    write!(
      f,
      "{}",
      self
        .format_string()
        .unwrap_or("{Unprintable Value}".to_string())
    )
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Value {
      string: Some(value),
      repr: Repr::None,
    }
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value {
      string: Some(value.to_string()),
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
