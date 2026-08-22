use reedline::{
  EditCommand, Emacs, KeyCode, KeyModifiers, Prompt, PromptEditMode, PromptHistorySearch,
  PromptHistorySearchStatus, Reedline, ReedlineEvent, Signal, ValidationResult, Validator,
  default_emacs_keybindings,
};
use std::borrow::Cow;
use std::sync::mpsc;
use std::thread;
use tanaid::eval::EvalContext;
use tanaid::parser::ParseError;
use tanaid::{eval, parser};
use tanaid_tk::Tk;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;

struct TclValidator;

impl Validator for TclValidator {
  fn validate(&self, line: &str) -> ValidationResult {
    match parser::parse(line) {
      Err(ParseError::Continuable(_)) => ValidationResult::Incomplete,
      _ => ValidationResult::Complete,
    }
  }
}

struct TclPrompt;

impl Prompt for TclPrompt {
  fn render_prompt_left(&self) -> Cow<'_, str> {
    Cow::Borrowed("tcl ")
  }

  fn render_prompt_right(&self) -> Cow<'_, str> {
    Cow::Borrowed("")
  }

  fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
    Cow::Borrowed("> ")
  }

  fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
    Cow::Borrowed("...   ")
  }

  fn render_prompt_history_search_indicator(
    &self,
    history_search: PromptHistorySearch,
  ) -> Cow<'_, str> {
    let prefix = match history_search.status {
      PromptHistorySearchStatus::Passing => "",
      PromptHistorySearchStatus::Failing => "failing ",
    };
    Cow::Owned(format!(
      "({}reverse-search: {}) ",
      prefix, history_search.term
    ))
  }
}

#[derive(Debug)]
enum ReplEvent {
  Line(String),
  Exit,
}

pub fn run_repl(
  context: &mut eval::EvalContext,
  tk: &mut Tk,
) -> Result<(), Box<dyn std::error::Error>> {
  let (next_tx, next_rx) = mpsc::channel::<()>();

  let event_loop = winit::event_loop::EventLoop::<ReplEvent>::with_user_event().build()?;
  let send_proxy = event_loop.create_proxy();

  let rl_thread = thread::spawn(move || {
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
      KeyModifiers::CONTROL,
      KeyCode::Char('c'),
      ReedlineEvent::ExecuteHostCommand("ctrl-c".to_string()),
    );

    let mut line_editor = Reedline::create()
      .with_edit_mode(Box::new(Emacs::new(keybindings)))
      .with_validator(Box::new(TclValidator {}));

    let prompt = TclPrompt {};

    loop {
      let line = match line_editor.read_line(&prompt) {
        Ok(Signal::Success(buffer)) => buffer,
        Ok(Signal::CtrlD) => {
          send_proxy.send_event(ReplEvent::Exit).unwrap();
          break;
        }
        Ok(Signal::HostCommand(command)) if command == "ctrl-c" => {
          line_editor.run_edit_commands(&[EditCommand::Clear]);
          println!();
          println!("ctrl+d to exit");
          continue;
        }
        _ => unimplemented!(),
      };

      send_proxy.send_event(ReplEvent::Line(line)).unwrap();
      next_rx.recv().unwrap();
    }
  });

  let mut app = ReplApp {
    tk,
    context,
    next_tx,
  };
  event_loop.run_app(&mut app)?;

  rl_thread.join().unwrap();

  Ok(())
}

struct ReplApp<'a> {
  tk: &'a mut Tk,
  context: &'a mut EvalContext,
  next_tx: mpsc::Sender<()>,
}

impl<'a> ApplicationHandler<ReplEvent> for ReplApp<'a> {
  fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: ReplEvent) {
    match event {
      ReplEvent::Exit => {
        event_loop.exit();
      }
      ReplEvent::Line(line) => {
        if let Err(err) = run_line(&line, &mut self.context) {
          println!("Error: {}", err);
        }
        self.next_tx.send(()).unwrap();
      }
    }
  }

  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.tk.context.handle_resumed(event_loop);
  }

  fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    self.tk.context.handle_about_to_wait(event_loop);
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

fn run_line(line: &str, context: &mut EvalContext) -> Result<(), Box<dyn std::error::Error>> {
  let parsed = parser::parse(line)?;
  let mut result = eval::eval(&parsed, context)?;
  println!("{}", result.repr_str()?);
  Ok(())
}
