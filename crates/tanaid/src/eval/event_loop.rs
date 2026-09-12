use std::{
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  time::Duration,
};

use crate::parser::ScriptNode;

type TimerId = usize;

pub struct EventLoop {
  timer_id: TimerId,
  pending_timers: HashMap<TimerId, ScriptNode>,
  timer_queue: BinaryHeap<Reverse<(Duration, TimerId)>>,
}

impl EventLoop {
  pub fn new() -> EventLoop {
    EventLoop {
      timer_id: 1,
      pending_timers: HashMap::new(),
      timer_queue: BinaryHeap::new(),
    }
  }

  pub fn start_timer(&mut self, now: Duration, callback: ScriptNode, delay: Duration) -> TimerId {
    let timer_id = self.timer_id;
    self.timer_id = self.timer_id.strict_add(1);
    self.timer_queue.push(Reverse((now + delay, timer_id)));
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
  pub fn take_elapsed(&mut self, now: Duration) -> Option<(TimerId, ScriptNode)> {
    loop {
      let Some(Reverse((next_fires_at, _))) = self.timer_queue.peek() else {
        break;
      };
      if *next_fires_at > now {
        break;
      }

      let Reverse((_, timer_id)) = self.timer_queue.pop().unwrap();
      if !self.pending_timers.contains_key(&timer_id) {
        // cancelled
        continue;
      }
      return Some(self.pending_timers.remove_entry(&timer_id).unwrap());
    }
    None
  }

  /// Compute the delay for the next pending timer.
  pub fn next_delay(&mut self, now: Duration) -> Option<Duration> {
    while let Some(Reverse((fires_at, timer_id))) = self.timer_queue.peek() {
      if self.pending_timers.contains_key(&timer_id) {
        return Some(fires_at.saturating_sub(now));
      }
      self.timer_queue.pop();
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use crate::eval::{EvalCmdResult, EvalContext};
  use crate::{eval, parser};
  use std::{cell::RefCell, rc::Rc};

  fn context_with_output() -> (EvalContext, Rc<RefCell<String>>) {
    let output = Rc::new(RefCell::new(String::new()));
    let output_sink = output.clone();
    let context = EvalContext::new()
      .with_std_time()
      .with_stdout(Rc::new(move |text| {
        output_sink.borrow_mut().push_str(text);
        Ok(())
      }));
    (context, output)
  }

  async fn eval_source(source: &str, context: &mut EvalContext) -> EvalCmdResult {
    eval::eval(&parser::parse(source).unwrap(), context).await
  }

  #[pollster::test]
  async fn poll_yields_before_newly_scheduled_timer() {
    let (mut context, output) = context_with_output();
    eval_source("after 0 {puts first; after 0 {puts second}}", &mut context)
      .await
      .unwrap();

    context.poll_event().await.unwrap();
    assert_eq!(*output.borrow(), "first\n");

    context.poll_event().await.unwrap();
    assert_eq!(*output.borrow(), "first\nsecond\n");
  }

  #[pollster::test]
  async fn poll_preserves_other_timers_after_error() {
    let (mut context, output) = context_with_output();
    eval_source(
      "after 0 {undefined_command}; after 0 {puts second}",
      &mut context,
    )
    .await
    .unwrap();

    assert!(context.poll_event().await.is_err());
    context.poll_event().await.unwrap();

    assert_eq!(*output.borrow(), "second\n");
  }
}
