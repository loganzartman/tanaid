#[derive(Debug)]
pub enum EvalError {
  Generic(String),
  ArgumentError(String),
  UndefinedCommand(String),
  UndefinedVariable(String),
  CommandParseError(String),
  ScriptParseError(String),
  ExprParseError(String),
  NotNumericError(String),
  BreakError,
  NotImplemented,
}

impl std::fmt::Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use EvalError::*;
    match self {
      Generic(s) => write!(f, "{}", s),
      ArgumentError(s) => write!(f, "Argument error: {}", s),
      UndefinedCommand(s) => write!(f, "Undefined command: {}", s),
      UndefinedVariable(s) => write!(f, "Undefined variable: {}", s),
      CommandParseError(s) => write!(f, "Failed to parse command: {}", s),
      ScriptParseError(s) => write!(f, "Failed to parse script: {}", s),
      ExprParseError(s) => write!(f, "Failed to parse expr: {}", s),
      NotNumericError(s) => write!(f, "Expected numeric value, got: {}", s),
      BreakError => write!(f, "Unexpected break command"),
      NotImplemented => write!(f, "Not implemented"),
    }
  }
}

impl std::error::Error for EvalError {}
