use clap::Parser;
use std::{
  fs,
  io::{self, IsTerminal},
  process::ExitCode,
};
use tanaid::event_loop;
use tanaid::{eval, parser};
use tanaid_cli::repl::run_repl;
use tanaid_tk::Tk;

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

  let mut tk = Tk::new();
  tk.install(&mut context)?;

  let opts = RunOpts { debug: args.debug };

  if let Some(file_path) = args.file_path {
    return run_source(
      fs::read_to_string(file_path)?.as_str(),
      &mut context,
      &mut tk,
      &opts,
    );
  }
  if io::stdin().is_terminal() {
    return run_repl(&mut context, &mut tk);
  }
  run_source(
    io::read_to_string(io::stdin())?.as_str(),
    &mut context,
    &mut tk,
    &opts,
  )
}

fn run_source(
  src: &str,
  context: &mut eval::EvalContext,
  tk: &mut Tk,
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

  let mut tcl_event_loop = event_loop::EventLoop::new();
  tcl_event_loop.run(context, || {
    tk.pump_app_events();
    Ok(())
  })?;

  Ok(())
}
