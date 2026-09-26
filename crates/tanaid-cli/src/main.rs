#![cfg_attr(target_family = "wasm", no_main)]
#![cfg(not(target_family = "wasm"))]

use clap::Parser;
use std::{
  cell::RefCell,
  fs,
  io::{self, IsTerminal},
  process::ExitCode,
  rc::Rc,
};
use tanaid::{
  eval,
  interpreter::{Interpreter, StepResult},
  parser,
};
use tanaid_cli::repl::run_repl;
use tanaid_tk::Tk;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;

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
  let mut context = eval::EvalContext::new().with_std_time();

  let mut tk = Tk::new();
  tk.install(&mut context)?;

  let opts = RunOpts { debug: args.debug };

  if let Some(file_path) = args.file_path {
    return run_source(fs::read_to_string(file_path)?.as_str(), context, tk, &opts);
  }
  if io::stdin().is_terminal() {
    return run_repl(context, tk);
  }
  run_source(
    io::read_to_string(io::stdin())?.as_str(),
    context,
    tk,
    &opts,
  )
}

fn run_source(
  src: &str,
  context: eval::EvalContext,
  mut tk: Tk,
  opts: &RunOpts,
) -> Result<(), Box<dyn std::error::Error>> {
  let parsed = parser::parse(src)?;
  if opts.debug {
    println!("=== parse tree ===");
    println!("{:#?}", parsed)
  }

  let mut interpreter = Interpreter::new();
  let tcl_event_loop = Rc::clone(&context.event_loop);
  interpreter.configure(context);

  interpreter.start(&parsed)?;

  // pump once to see if window is opened or events have been queued.
  // if script returns synchronously and there's no window open, we're done.
  if let StepResult::Done(mut value) = interpreter.step()? {
    println!("{}", value.repr_str()?);
    // by convention, we keep running if a window is open to generate events,
    // even if the script didn't include any explicit wait.
    if !tk.context.has_window() {
      return Ok(());
    }
  }

  let mut app = SourceApp {
    tk: &mut tk,
    interpreter,
    tcl_event_loop,
    error: None,
  };

  let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;
  event_loop.run_app(&mut app)?;

  match app.error {
    Some(err) => Err(err),
    None => Ok(()),
  }
}

struct SourceApp<'a> {
  tk: &'a mut Tk,
  interpreter: Interpreter,
  tcl_event_loop: Rc<RefCell<tanaid::event_loop::EventLoop>>,
  error: Option<Box<dyn std::error::Error>>,
}

impl<'a> ApplicationHandler for SourceApp<'a> {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.tk.context.handle_resumed(event_loop);
  }

  fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.tk.context.handle_about_to_wait(event_loop);

    match self.interpreter.step() {
      Err(err) => {
        println!("Error: {}", err);
        event_loop.exit();
      }
      Ok(StepResult::Again) => event_loop.set_control_flow(ControlFlow::Poll),
      Ok(StepResult::WaitDuration(duration)) => {
        event_loop.set_control_flow(ControlFlow::wait_duration(duration))
      }
      Ok(StepResult::Wait) => event_loop.set_control_flow(ControlFlow::Wait),
      Ok(StepResult::Done(_)) => event_loop.set_control_flow(ControlFlow::Poll),
    }

    if !self.interpreter.is_busy() && !self.tk.context.has_window() {
      event_loop.exit();
    }
  }

  fn window_event(
    &mut self,
    _event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    self
      .tk
      .context
      .handle_window_event(window_id, event, self.tcl_event_loop.clone());
  }
}
