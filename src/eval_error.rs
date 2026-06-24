#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  UndefinedVariable(String),
  CommandParseError(String),
  ExprParseError(String),
  NotNumericError(String),
  NotImplemented,
}

impl std::fmt::Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use EvalError::*;
    match self {
      Generic(s) => write!(f, "{}", s),
      UndefinedVariable(s) => write!(f, "Undefined variable: {}", s),
      CommandParseError(s) => write!(f, "Failed to parse command: {}", s),
      ExprParseError(s) => write!(f, "Failed to parse expr: {}", s),
      NotNumericError(s) => write!(f, "Expected numeric value, got: {}", s),
      NotImplemented => write!(f, "Not implemented"),
    }
  }
}

impl std::error::Error for EvalError {}
