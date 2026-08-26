use clap::Parser;
use std::{
  fs,
  io::{self, IsTerminal},
  process::ExitCode,
};
use tanaid::{eval, parser};
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

  let mut tcl_event_loop = tanaid::event_loop::EventLoop::new();
  tcl_event_loop.apply_actions(context.take_timer_actions());

  // nothing left to do: no pending timers and no window to service
  if tcl_event_loop.count_pending() == 0 && !tk.context.has_window() {
    return Ok(());
  }

  let event_loop = winit::event_loop::EventLoop::new()?;
  let mut app = SourceApp {
    tk,
    context,
    tcl_event_loop,
    had_window: false,
    error: None,
  };
  event_loop.run_app(&mut app)?;

  match app.error {
    Some(err) => Err(err),
    None => Ok(()),
  }
}

struct SourceApp<'a> {
  tk: &'a mut Tk,
  context: &'a mut eval::EvalContext,
  tcl_event_loop: tanaid::event_loop::EventLoop,
  had_window: bool,
  error: Option<Box<dyn std::error::Error>>,
}

impl<'a> ApplicationHandler for SourceApp<'a> {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.tk.context.handle_resumed(event_loop);
  }

  fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    if let Err(err) = self.tcl_event_loop.poll(self.context) {
      self.error = Some(Box::new(err));
      event_loop.exit();
      return;
    }

    self.tk.context.handle_about_to_wait(event_loop);

    // like wish: once a window has been opened, it alone holds the loop open
    if self.tk.context.has_window() {
      self.had_window = true;
    } else if self.had_window {
      event_loop.exit();
      return;
    }

    let next_deadline = self.tcl_event_loop.next_deadline();
    if next_deadline.is_none() && !self.had_window {
      event_loop.exit();
      return;
    }

    match next_deadline {
      Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
      None => event_loop.set_control_flow(ControlFlow::Wait),
    }
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    self
      .tk
      .context
      .handle_window_event(event_loop, window_id, event);
  }
}
