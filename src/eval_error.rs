#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  UndefinedVariable(String),
  CommandParseError(String),
  ExprParseError(String),
  NotImplemented,
}

impl std::fmt::Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use EvalError::*;
    match self {
      Generic(s) => write!(f, "{}", s),
      UndefinedVariable(v) => write!(f, "Undefined variable: {}", v),
      CommandParseError(e) => write!(f, "Failed to parse command: {}", e),
      ExprParseError(e) => write!(f, "Failed to parse expr: {}", e),
      NotImplemented => write!(f, "Not implemented"),
    }
  }
}

impl std::error::Error for EvalError {}
