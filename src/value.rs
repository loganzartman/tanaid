use crate::eval_error::EvalError;
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
  string: String,
  repr: Repr,
}

impl Value {
  pub fn none() -> Value {
    Value {
      string: "".to_string(),
      repr: Repr::None,
    }
  }

  pub fn new(x: impl Into<String>) -> Value {
    Value {
      string: x.into(),
      repr: Repr::None,
    }
  }

  pub fn repr_str(&mut self) -> Result<&str, EvalError> {
    // TODO: lazy materialize
    Ok(&self.string)
  }

  pub fn repr_int(&mut self) -> Result<i64, EvalError> {
    if let Repr::Int(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .string
      .parse::<i64>()
      .map_err(|e| EvalError::Generic(e.to_string()))?;
    self.repr = Repr::Int(x);
    Ok(x)
  }

  pub fn repr_float(&mut self) -> Result<f64, EvalError> {
    if let Repr::Float(x) = self.repr {
      return Ok(x);
    }

    let x = self
      .string
      .parse::<f64>()
      .map_err(|e| EvalError::Generic(e.to_string()))?;
    self.repr = Repr::Float(x);
    Ok(x)
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Value {
      string: value,
      repr: Repr::None,
    }
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value {
      string: value.to_string(),
      repr: Repr::None,
    }
  }
}

impl From<i64> for Value {
  fn from(value: i64) -> Self {
    Value {
      string: value.to_string(),
      repr: Repr::Int(value),
    }
  }
}

impl From<f64> for Value {
  fn from(value: f64) -> Self {
    Value {
      string: value.to_string(),
      repr: Repr::Float(value),
    }
  }
}

impl ops::Add for Value {
  type Output = Result<Value, EvalError>;

  fn add(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.repr_int(), rhs.repr_int()) {
      return Ok(Value::from(a + b));
    }
    Ok(Value::from(self.repr_float()? + rhs.repr_float()?))
  }
}

impl ops::Sub for Value {
  type Output = Result<Value, EvalError>;

  fn sub(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.repr_int(), rhs.repr_int()) {
      return Ok(Value::from(a - b));
    }
    Ok(Value::from(self.repr_float()? - rhs.repr_float()?))
  }
}

impl ops::Mul for Value {
  type Output = Result<Value, EvalError>;

  fn mul(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.repr_int(), rhs.repr_int()) {
      return Ok(Value::from(a * b));
    }
    Ok(Value::from(self.repr_float()? * rhs.repr_float()?))
  }
}

impl ops::Div for Value {
  type Output = Result<Value, EvalError>;

  fn div(mut self, mut rhs: Self) -> Self::Output {
    if let (Ok(a), Ok(b)) = (self.repr_int(), rhs.repr_int()) {
      return Ok(Value::from(a / b));
    }
    Ok(Value::from(self.repr_float()? / rhs.repr_float()?))
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.string)
  }
}
