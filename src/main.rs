use clap::Parser;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use std::{
  fs,
  io::{self, IsTerminal},
};
use tanaid::value::Value;
use tanaid::{eval, parser};

#[derive(Parser, Debug)]
struct Args {
  file_path: Option<String>,
  debug: Option<bool>,
}

struct RunOpts {
  debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let mut context = eval::EvalContext::new();
  let opts = RunOpts {
    debug: args.debug.unwrap_or(false),
  };

  if let Some(file_path) = args.file_path {
    return run_source(fs::read_to_string(file_path)?.as_str(), &mut context, &opts);
  }
  if io::stdin().is_terminal() {
    return run_repl(&mut context, &opts);
  }
  run_source(
    io::read_to_string(io::stdin())?.as_str(),
    &mut context,
    &opts,
  )
}

fn run_repl(
  context: &mut eval::EvalContext,
  opts: &RunOpts,
) -> Result<(), Box<dyn std::error::Error>> {
  let mut line_editor = Reedline::create();
  let prompt = DefaultPrompt {
    left_prompt: DefaultPromptSegment::Basic("tcl ".to_string()),
    right_prompt: DefaultPromptSegment::Empty,
  };

  loop {
    let sig = line_editor.read_line(&prompt);
    match sig {
      Ok(Signal::Success(buffer)) => {
        let mut result = exec(buffer.as_str(), context, opts)?;
        println!("{}", result.repr_str()?);
      }
      Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
        println!("see ya!");
        break;
      }
      _ => unimplemented!(),
    }
  }

  Ok(())
}

fn run_source(
  src: &str,
  context: &mut eval::EvalContext,
  opts: &RunOpts,
) -> Result<(), Box<dyn std::error::Error>> {
  let mut result = exec(src, context, opts)?;
  println!("{}", result.repr_str()?);
  Ok(())
}

fn exec(
  src: &str,
  context: &mut eval::EvalContext,
  opts: &RunOpts,
) -> Result<Value, Box<dyn std::error::Error>> {
  let parsed = parser::parse(src)?;
  if opts.debug {
    println!("{:#?}", parsed)
  }

  let result = eval::eval(parsed, context)?;
  if opts.debug {
    println!("{:#?}", result);
  }

  Ok(result)
}

#[cfg(test)]
mod tests {
  #[test]
  fn greet_returns_hello() {}
}
