use crate::{eval::EvalContext, eval_error::EvalError, value::Value};

pub fn register_commands(context: &mut EvalContext) {
  context.register_command("::tcl::mathfunc::abs", |args, _, _| {
    let [x] = args else {
      return Err(EvalError::ArgumentError(
        "abs expects 1 argument".to_string(),
      ));
    };
    if let Some(x) = x.repr_int().ok().and_then(|x| x.checked_abs()) {
      return Ok(x.into());
    }
    if let Ok(x) = x.repr_float() {
      return Ok(x.abs().into());
    }
    Err(EvalError::ArgumentError(format!(
      "expected number but got: {}",
      x.to_string()
    )))
  });

  context.register_command("::tcl::mathfunc::fmod", |args, _, _| {
    let [x, y] = args else {
      return Err(EvalError::ArgumentError(
        "fmod expects 2 arguments".to_string(),
      ));
    };

    let Ok(x) = x.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        x.to_string(),
      )));
    };
    let Ok(y) = y.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        y.to_string(),
      )));
    };

    let rem = x % y;
    if rem.is_nan() {
      Err(EvalError::Generic(
        "argument not in valid range".to_string(),
      ))
    } else {
      Ok(rem.into())
    }
  });

  context.register_command("::tcl::mathfunc::srand", |args, context, _| {
    let [seed] = args else {
      return Err(EvalError::ArgumentError(
        "srand expects 1 argument".to_string(),
      ));
    };

    // https://github.com/tcltk/tcl/blob/102a512d4f61d457a239d8c56f1514bffb52d08f/generic/tclBasic.c#L7966
    let state = normalize_rand_seed(seed.repr_int()? as i32);
    context.random_state = Some(state);

    mathfunc_rand(context)
  });

  context.register_command("::tcl::mathfunc::rand", |args, context, _| {
    if !args.is_empty() {
      return Err(EvalError::ArgumentError(
        "rand expects 0 arguments".to_string(),
      ));
    };

    mathfunc_rand(context)
  });
}

fn mathfunc_rand(context: &mut EvalContext) -> Result<Value, EvalError> {
  // https://github.com/tcltk/tcl/blob/102a512d4f61d457a239d8c56f1514bffb52d08f/generic/tclBasic.c#L7048
  const IA: i32 = 16807;
  const IM: i32 = 2147483647;
  const IQ: i32 = 127773;
  const IR: i32 = 2836;

  let mut state = match context.random_state {
    None => {
      let seed = normalize_rand_seed(context.clock_unixtime()?.as_millis() as i32);
      context.random_state = Some(seed);
      seed
    }
    Some(state) => state,
  };

  let tmp: i32 = state / IQ;
  state = IA * (state - tmp * IQ) - IR * tmp;

  if state < 0 {
    state += IM;
  }
  context.random_state = Some(state);

  Ok(Value::from(state as f64 * (1.0 / IM as f64)))
}

fn normalize_rand_seed(mut seed: i32) -> i32 {
  seed = seed & 0x7fffffff;
  if seed == 0 || seed == 0x7fffffff {
    seed ^= 123459876;
  }
  seed
}
