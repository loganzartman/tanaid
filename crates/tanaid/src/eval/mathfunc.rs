use crate::{
  eval::{EvalContext, FrameId},
  eval_error::EvalError,
  value::Value,
};

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

  context.register_command(
    "::tcl::mathfunc::acos",
    mf_float1_1("acos", |x| Ok(x.acos())),
  );

  context.register_command(
    "::tcl::mathfunc::asin",
    mf_float1_1("asin", |x| Ok(x.asin())),
  );

  context.register_command(
    "::tcl::mathfunc::atan",
    mf_float1_1("atan", |x| Ok(x.atan())),
  );

  context.register_command(
    "::tcl::mathfunc::atan2",
    mf_float2_1("atan2", |x, y| Ok(x.atan2(y))),
  );

  context.register_command(
    "::tcl::mathfunc::ceil",
    mf_float1_1("ceil", |x| Ok(x.ceil())),
  );

  context.register_command("::tcl::mathfunc::cos", mf_float1_1("cos", |x| Ok(x.cos())));

  context.register_command(
    "::tcl::mathfunc::cosh",
    mf_float1_1("cosh", |x| Ok(x.cosh())),
  );

  context.register_command("::tcl::mathfunc::double", mf_float1_1("double", |x| Ok(x)));

  // TODO: arbitrary precision
  context.register_command(
    "::tcl::mathfunc::entier",
    mf_int1_1("entier", |x| Ok(x), |x| x.trunc() as i64),
  );

  context.register_command("::tcl::mathfunc::exp", mf_float1_1("exp", |x| Ok(x.exp())));

  context.register_command(
    "::tcl::mathfunc::floor",
    mf_float1_1("floor", |x| Ok(x.floor())),
  );

  context.register_command(
    "::tcl::mathfunc::fmod",
    mf_float2_1("fmod", |x, y| Ok(x % y)),
  );

  context.register_command(
    "::tcl::mathfunc::hypot",
    mf_float2_1("hypot", |x, y| Ok(x.hypot(y))),
  );

  context.register_command(
    "::tcl::mathfunc::int",
    mf_int1_1("int", |x| Ok(x), |x| x.trunc() as i64),
  );

  context.register_command(
    "::tcl::mathfunc::isqrt",
    mf_int1_1(
      "isqrt",
      |x| {
        x.checked_isqrt()
          .ok_or_else(|| EvalError::ArgumentError("argument not in range".to_string()))
      },
      |x| x.trunc() as i64,
    ),
  );

  context.register_command("::tcl::mathfunc::log", mf_float1_1("log", |x| Ok(x.ln())));

  context.register_command(
    "::tcl::mathfunc::log10",
    mf_float1_1("log10", |x| Ok(x.log10())),
  );

  context.register_command(
    "::tcl::mathfunc::pow",
    mf_float2_1("pow", |x, y| Ok(x.powf(y))),
  );

  context.register_command("::tcl::mathfunc::rand", |args, context, _| {
    if !args.is_empty() {
      return Err(EvalError::ArgumentError(
        "rand expects 0 arguments".to_string(),
      ));
    };

    rand(context)
  });

  context.register_command(
    "::tcl::mathfunc::round",
    mf_int1_1("round", |x| Ok(x), |x| x.round() as i64),
  );

  context.register_command("::tcl::mathfunc::sin", mf_float1_1("sin", |x| Ok(x.sin())));

  context.register_command(
    "::tcl::mathfunc::sinh",
    mf_float1_1("sinh", |x| Ok(x.sinh())),
  );

  context.register_command(
    "::tcl::mathfunc::sqrt",
    mf_float1_1("sqrt", |x| Ok(x.sqrt())),
  );

  context.register_command("::tcl::mathfunc::srand", |args, context, _| {
    let [seed] = args else {
      return Err(EvalError::ArgumentError(
        "srand expects 1 argument".to_string(),
      ));
    };

    // https://github.com/tcltk/tcl/blob/102a512d4f61d457a239d8c56f1514bffb52d08f/generic/tclBasic.c#L7966
    let state = normalize_rand_seed(seed.repr_int()? as i32);
    context.random_state = Some(state);

    rand(context)
  });

  context.register_command("::tcl::mathfunc::tan", mf_float1_1("tan", |x| Ok(x.tan())));

  context.register_command(
    "::tcl::mathfunc::tanh",
    mf_float1_1("tanh", |x| Ok(x.tanh())),
  );

  context.register_command(
    "::tcl::mathfunc::wide",
    mf_int1_1("wide", |x| Ok(x), |x| x.trunc() as i64),
  );
}

fn rand(context: &mut EvalContext) -> Result<Value, EvalError> {
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

fn mf_int1_1(
  name: &str,
  f: impl Fn(i64) -> Result<i64, EvalError> + 'static,
  to_int: impl Fn(f64) -> i64 + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  let arg_error = format!("{} expects 1 argument", name);

  move |args, _, _| {
    let [x] = args else {
      return Err(EvalError::ArgumentError(arg_error.clone()));
    };
    if let Ok(x) = x.repr_int() {
      return Ok(f(x)?.into());
    }
    let Ok(x) = x.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        x.to_string()
      )));
    };
    Ok(f(to_int(x))?.into())
  }
}

fn mf_float1_1(
  name: &str,
  f: impl Fn(f64) -> Result<f64, EvalError> + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  let arg_error = format!("{} expects 1 argument", name);

  move |args, _, _| {
    let [x] = args else {
      return Err(EvalError::ArgumentError(arg_error.clone()));
    };
    let Ok(x) = x.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        x.to_string()
      )));
    };
    return Ok(f(x)?.into());
  }
}

fn mf_float2_1(
  name: &str,
  f: impl Fn(f64, f64) -> Result<f64, EvalError> + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  let arg_error = format!("{} expects 2 arguments", name);

  move |args, _, _| {
    let [x, y] = args else {
      return Err(EvalError::ArgumentError(arg_error.clone()));
    };
    let Ok(x) = x.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        x.to_string()
      )));
    };
    let Ok(y) = y.repr_float() else {
      return Err(EvalError::ArgumentError(format!(
        "expected number but got: {}",
        y.to_string()
      )));
    };
    return Ok(f(x, y)?.into());
  }
}
