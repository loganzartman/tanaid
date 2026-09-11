use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  time::{Duration, Instant},
};

use crate::{eval_error::EvalError, parser::ScriptNode};

type TimerId = usize;

pub struct EventLoop {
  timer_id: TimerId,
  pending_timers: HashMap<TimerId, ScriptNode>,
  timer_queue: BinaryHeap<Reverse<(Instant, TimerId)>>,
}

impl EventLoop {
  pub fn new() -> EventLoop {
    EventLoop {
      timer_id: 1,
      pending_timers: HashMap::new(),
      timer_queue: BinaryHeap::new(),
    }
  }

  pub fn start_timer(&mut self, callback: ScriptNode, delay_ms: u64) -> TimerId {
    let timer_id = self.timer_id;
    self.timer_id = self.timer_id.strict_add(1);
    self.timer_queue.push(Reverse((
      Instant::now() + Duration::from_millis(delay_ms),
      timer_id,
    )));
    self.pending_timers.insert(timer_id, callback);
    timer_id
  }

  pub fn cancel_timer(&mut self, timer_id: TimerId) {
    self.pending_timers.remove(&timer_id);
  }

  pub fn count_pending(&self) -> usize {
    self.pending_timers.len()
  }

  /// Take the next elapsed timer, if any.
  pub fn take_elapsed(&mut self) -> Result<Option<(TimerId, ScriptNode)>, EvalError> {
    while self
      .timer_queue
      .peek()
      .is_some_and(|Reverse((fires_at, _))| *fires_at <= Instant::now())
    {
      let Reverse((_, timer_id)) = self.timer_queue.pop().unwrap();
      if !self.pending_timers.contains_key(&timer_id) {
        // cancelled
        continue;
      }
      return Ok(Some(self.pending_timers.remove_entry(&timer_id).unwrap()));
    }
    Ok(None)
  }

  /// Compute the deadline for the next pending timer.
  pub fn next_deadline(&mut self) -> Option<Instant> {
    while let Some(Reverse((fires_at, timer_id))) = self.timer_queue.peek() {
      if self.pending_timers.contains_key(&timer_id) {
        return Some(fires_at.clone());
      }
      self.timer_queue.pop();
    }

    None
  }
}

#[cfg(test)]
mod tests {
  use crate::eval::EvalContext;
  use crate::{eval, parser};
  use std::{cell::RefCell, rc::Rc};

  fn context_with_output() -> (EvalContext, Rc<RefCell<String>>) {
    let output = Rc::new(RefCell::new(String::new()));
    let output_sink = output.clone();
    let context = EvalContext::new().with_stdout(Rc::new(move |text| {
      output_sink.borrow_mut().push_str(text);
      Ok(())
    }));
    (context, output)
  }

  fn eval_source(source: &str, context: &mut EvalContext) {
    eval::eval(&parser::parse(source).unwrap(), context).unwrap();
  }

  #[test]
  fn poll_yields_before_newly_scheduled_timer() {
    let (mut context, output) = context_with_output();
    eval_source("after 0 {puts first; after 0 {puts second}}", &mut context);

    context.poll_events().unwrap();
    assert_eq!(*output.borrow(), "first\n");

    context.poll_events().unwrap();
    assert_eq!(*output.borrow(), "first\nsecond\n");
  }

  #[test]
  fn poll_preserves_other_timers_after_error() {
    let (mut context, output) = context_with_output();
    eval_source(
      "after 0 {undefined_command}; after 0 {puts second}",
      &mut context,
    );

    assert!(context.poll_events().is_err());
    context.poll_events().unwrap();

    assert_eq!(*output.borrow(), "second\n");
  }
}
