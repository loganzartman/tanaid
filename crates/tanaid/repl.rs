use crate::parser::ParseError;
use crate::{eval, parser};
use reedline::{
  EditCommand, Emacs, KeyCode, KeyModifiers, Prompt, PromptEditMode, PromptHistorySearch,
  PromptHistorySearchStatus, Reedline, ReedlineEvent, Signal, ValidationResult, Validator,
  default_emacs_keybindings,
};
use std::borrow::Cow;

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

pub fn run_repl(context: &mut eval::EvalContext) -> Result<(), Box<dyn std::error::Error>> {
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
        return Ok(());
      }
      Ok(Signal::HostCommand(command)) if command == "ctrl-c" => {
        line_editor.run_edit_commands(&[EditCommand::Clear]);
        println!();
        println!("ctrl+d to exit");
        continue;
      }
      _ => unimplemented!(),
    };

    let parsed = match parser::parse(line.as_str()) {
      Ok(parsed) => parsed,
      Err(err) => {
        println!("Error: {}", err);
        continue;
      }
    };

    match eval::eval(&parsed, context) {
      Ok(mut result) => {
        println!("{}", result.repr_str()?);
      }
      Err(err) => {
        println!("Error: {}", err);
      }
    }
  }
}
