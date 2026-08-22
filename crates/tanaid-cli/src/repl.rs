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

pub fn run_repl(
  context: &mut eval::EvalContext,
  _tk: &mut Tk,
) -> Result<(), Box<dyn std::error::Error>> {
  let (lines_tx, lines_rx) = mpsc::channel();
  let (next_tx, next_rx) = mpsc::channel();

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
          return;
        }
        Ok(Signal::HostCommand(command)) if command == "ctrl-c" => {
          line_editor.run_edit_commands(&[EditCommand::Clear]);
          println!();
          println!("ctrl+d to exit");
          continue;
        }
        _ => unimplemented!(),
      };

      lines_tx.send(line).unwrap();
      next_rx.recv().unwrap();
    }
  });

  for line in lines_rx.iter() {
    if let Err(err) = run_line(line.as_str(), context) {
      println!("Error: {}", err);
    }
    next_tx.send(()).unwrap();
  }

  rl_thread.join().unwrap();

  Ok(())
}

fn run_line(line: &str, context: &mut EvalContext) -> Result<(), Box<dyn std::error::Error>> {
  let parsed = parser::parse(line)?;
  let mut result = eval::eval(&parsed, context)?;
  println!("{}", result.repr_str()?);
  Ok(())
}
