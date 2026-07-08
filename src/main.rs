use clap::Parser;
use std::{
  fs,
  io::{self, IsTerminal},
  process::ExitCode,
};
use tanaid::repl::run_repl;
use tanaid::{eval, parser};

#[derive(Parser, Debug)]
struct Args {
  file_path: Option<String>,

  #[arg(short, long, default_value_t = false)]
  debug: bool,
}

struct RunOpts {
  debug: bool,
}

fn main() -> ExitCode {
  match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(err) => {
      eprintln!("Error: {}", err);
      ExitCode::FAILURE
    }
  }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let mut context = eval::EvalContext::new();
  let opts = RunOpts { debug: args.debug };

  if let Some(file_path) = args.file_path {
    return run_source(fs::read_to_string(file_path)?.as_str(), &mut context, &opts);
  }
  if io::stdin().is_terminal() {
    return run_repl(&mut context);
  }
  run_source(
    io::read_to_string(io::stdin())?.as_str(),
    &mut context,
    &opts,
  )
}

fn run_source(
  src: &str,
  context: &mut eval::EvalContext,
  opts: &RunOpts,
) -> Result<(), Box<dyn std::error::Error>> {
  let parsed = parser::parse(src)?;
  if opts.debug {
    println!("=== parse tree ===");
    println!("{:#?}", parsed)
  }

  let mut result = eval::eval(&parsed, context)?;
  if opts.debug {
    println!("=== result ===");
    println!("{:#?}", result);
  }

  println!("{}", result.repr_str()?);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = eval::EvalContext::new();
    let opts = RunOpts { debug: true };
    run_source(
      fs::read_to_string("./sample/fib.tcl")?.as_str(),
      &mut context,
      &opts,
    )
  }
}
