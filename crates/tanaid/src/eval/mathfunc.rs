use std::cmp::Ordering;

use crate::{
  eval::{EvalContext, FrameId},
  eval_error::EvalError,
  value::{Number, Value},
};

pub fn register_commands(context: &mut EvalContext) {
  context.register_command("::tcl::mathfunc::abs", |args, _, _| {
    let [x] = args else {
      return Err(arity_error("abs", 1, args.len()));
    };
    match x.repr_number() {
      Ok(Number::Int(x)) => Ok(
        x.checked_abs()
          .map(Value::from)
          .unwrap_or_else(|| (x as f64).abs().into()),
      ),
      Ok(Number::Float(x)) => Ok(x.abs().into()),
      Err(_) => Err(type_error("number", x)),
    }
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

  context.register_command("::tcl::mathfunc::bool", |args, _, _| {
    let [x] = args else {
      return Err(arity_error("bool", 1, args.len()));
    };
    Ok(x.repr_bool()?.into())
  });

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
          .ok_or_else(|| EvalError::ArgumentError("square root of negative argument".to_string()))
      },
      |x| x.floor() as i64,
    ),
  );

  context.register_command("::tcl::mathfunc::log", mf_float1_1("log", |x| Ok(x.ln())));

  context.register_command(
    "::tcl::mathfunc::log10",
    mf_float1_1("log10", |x| Ok(x.log10())),
  );

  context.register_command("::tcl::mathfunc::max", mf_min_max("max", Ordering::Greater));

  context.register_command("::tcl::mathfunc::min", mf_min_max("min", Ordering::Less));

  context.register_command(
    "::tcl::mathfunc::pow",
    mf_float2_1("pow", |x, y| Ok(x.powf(y))),
  );

  context.register_command("::tcl::mathfunc::rand", |args, context, _| {
    if !args.is_empty() {
      return Err(arity_error("rand", 0, args.len()));
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
      return Err(arity_error("srand", 1, args.len()));
    };
    let Ok(seed) = seed.repr_int() else {
      return Err(type_error("integer", seed));
    };

    // https://github.com/tcltk/tcl/blob/102a512d4f61d457a239d8c56f1514bffb52d08f/generic/tclBasic.c#L7966
    let state = normalize_rand_seed(seed as i32);
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

fn arity_error(name: &str, expected: usize, got: usize) -> EvalError {
  let problem = if got < expected {
    "not enough"
  } else {
    "too many"
  };
  EvalError::ArgumentError(format!("{problem} arguments for math function \"{name}\""))
}

fn type_error(expected: &str, got: &Value) -> EvalError {
  EvalError::ArgumentError(format!("expected {expected} but got \"{got}\""))
}

fn mf_int1_1(
  name: &'static str,
  f: impl Fn(i64) -> Result<i64, EvalError> + 'static,
  to_int: impl Fn(f64) -> i64 + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  move |args, _, _| {
    let [x] = args else {
      return Err(arity_error(name, 1, args.len()));
    };
    match x.repr_number() {
      Ok(Number::Int(x)) => Ok(f(x)?.into()),
      Ok(Number::Float(x)) => Ok(f(to_int(x))?.into()),
      Err(_) => Err(type_error("number", x)),
    }
  }
}

fn mf_float1_1(
  name: &'static str,
  f: impl Fn(f64) -> Result<f64, EvalError> + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  move |args, _, _| {
    let [x] = args else {
      return Err(arity_error(name, 1, args.len()));
    };
    let Ok(x) = x.repr_float() else {
      return Err(type_error("floating-point number", x));
    };
    Ok(f(x)?.into())
  }
}

fn mf_float2_1(
  name: &'static str,
  f: impl Fn(f64, f64) -> Result<f64, EvalError> + 'static,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  move |args, _, _| {
    let [x, y] = args else {
      return Err(arity_error(name, 2, args.len()));
    };
    let Ok(x) = x.repr_float() else {
      return Err(type_error("floating-point number", x));
    };
    let Ok(y) = y.repr_float() else {
      return Err(type_error("floating-point number", y));
    };
    Ok(f(x, y)?.into())
  }
}

fn mf_min_max(
  name: &'static str,
  ordering: Ordering,
) -> impl Fn(&mut [Value], &mut EvalContext, FrameId) -> Result<Value, EvalError> + 'static {
  move |args, _, _| {
    let Some((mut result, rest)) = args.split_first_mut() else {
      return Err(arity_error(name, 1, args.len()));
    };
    if !result.is_numeric() {
      return Err(type_error("number", result));
    }

    for arg in rest {
      if !arg.is_numeric() {
        return Err(type_error("number", arg));
      }
      if arg.compare(result)? == Some(ordering) {
        result = arg;
      }
    }

    Ok(result.clone())
  }
}
