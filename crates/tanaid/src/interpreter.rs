use crate::eval::{self, EvalContext};
use crate::eval_error::EvalError;
use crate::event_loop::{EventLoop, EventWait};
use crate::parser::ScriptNode;
use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

// flow:
// user creates an Interpreter
// the Interpreter is a long-lived handle that allows for integration with the host event loop
// user creates an EvalContext (can bind variables, register commands)
// user hands over the EvalContext to Interpreter via configure()
// user can run() code with a configure()'d Interpreter, and await the output.
// once the output is awaited, more code can be run().
// meanwhile, an external event loop (winit, etc.) can call and handle step(), which runs the Tcl event loop

pub struct Interpreter {
  state: InterpreterState,
}

enum InterpreterState {
  Init,
  Configured(Rc<RefCell<EventLoop>>, EvalContext),
  Running(Rc<RefCell<EventLoop>>),
}

impl Interpreter {
  pub fn new() -> Self {
    Interpreter {
      state: InterpreterState::Init,
    }
  }

  pub async fn step(&mut self) -> Result<EventWait, EvalError> {
    match &mut self.state {
      InterpreterState::Init => Ok(EventWait::Idle),
      InterpreterState::Configured(event_loop, context) => {
        context.poll_event().await?;
        event_loop.borrow_mut().next_wait()
      }
      InterpreterState::Running(event_loop) => event_loop.borrow_mut().next_wait(),
    }
  }

  pub fn configure(&mut self, context: EvalContext) {
    self.state = InterpreterState::Configured(Rc::clone(&context.event_loop), context);
  }

  pub async fn run(&mut self, script: &ScriptNode) -> Result<Value, EvalError> {
    let mut context = match std::mem::replace(&mut self.state, InterpreterState::Init) {
      InterpreterState::Init => {
        return Err(EvalError::Generic(
          "interpreter not configured with EvalContext".to_string(),
        ));
      }
      InterpreterState::Running(_) => {
        return Err(EvalError::Generic(
          "interpreter already running".to_string(),
        ));
      }
      InterpreterState::Configured(_, context) => context,
    };

    self.state = InterpreterState::Running(Rc::clone(&context.event_loop));

    let script = script.clone();
    let result = eval::eval(&script, &mut context).await;

    self.state = InterpreterState::Configured(Rc::clone(&context.event_loop), context);

    Ok(result?)
  }
}
