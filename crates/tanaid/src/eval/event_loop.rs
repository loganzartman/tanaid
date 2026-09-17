use std::{
  cell::RefCell,
  cmp::Reverse,
  collections::{BinaryHeap, HashMap},
  pin::Pin,
  rc::Rc,
  task::{Poll, Waker},
  time::Duration,
};

use crate::{eval::EvalContext, eval_error::EvalError};

pub type EventId = usize;

pub enum EventWait {
  /// An event is due to be dispatched
  Ready,
  /// An event is scheduled after this delay (suspend for this long)
  Delay(Duration),
  /// No events are pending; loop is idle (suspend indefinitely)
  Idle,
}

pub trait Event {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut EvalContext,
  ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>>;
}

pub struct EventLoop {
  pub(crate) clock_monotonic: Option<Rc<dyn Fn() -> Duration + 'static>>,
  event_id: EventId,
  pending_events: HashMap<EventId, Box<dyn Event>>,
  event_queue: BinaryHeap<Reverse<(Duration, EventId)>>,
  waker: Option<Waker>,
}

impl EventLoop {
  pub fn new() -> EventLoop {
    EventLoop {
      clock_monotonic: None,
      event_id: 1,
      pending_events: HashMap::new(),
      event_queue: BinaryHeap::new(),
      waker: None,
    }
  }

  fn clock_monotonic(&self) -> Result<Duration, EvalError> {
    let Some(clock_monotonic) = &self.clock_monotonic else {
      return Err(EvalError::Generic(
        "EventLoop missing clock_monotonic".to_string(),
      ));
    };

    Ok(clock_monotonic())
  }

  fn wake(&mut self) {
    if let Some(waker) = self.waker.take() {
      waker.wake();
    }
  }

  /// enqueue an event to dispatch at a given deadline
  pub fn push_scheduled(&mut self, event: Box<dyn Event>, deadline: Duration) -> EventId {
    let event_id = self.event_id;
    self.event_id += 1;

    self.pending_events.insert(event_id, event);
    self.event_queue.push(Reverse((deadline, event_id)));
    self.wake();
    event_id
  }

  /// enqueue an event to dispatch ASAP, before any timers
  pub fn push_immediate(&mut self, event: Box<dyn Event>) -> EventId {
    self.push_scheduled(event, Duration::ZERO)
  }

  /// remove an event from the queue
  pub fn remove(&mut self, event_id: EventId) {
    self.pending_events.remove(&event_id);
  }

  /// get the number of queued events
  pub fn count_pending(&self) -> usize {
    self.pending_events.len()
  }

  /// determine whether an event is ready, or how long to wait for one
  pub fn next_wait(&mut self) -> Result<EventWait, EvalError> {
    while let Some(Reverse((deadline, event_id))) = self.event_queue.peek() {
      if !self.pending_events.contains_key(&event_id) {
        // cancelled
        self.event_queue.pop();
        continue;
      }

      if &self.clock_monotonic()? >= deadline {
        return Ok(EventWait::Ready);
      }
      return Ok(EventWait::Delay(
        deadline.saturating_sub(self.clock_monotonic()?),
      ));
    }
    Ok(EventWait::Idle)
  }

  /// take the next ready event, if any.
  pub fn take_ready(&mut self) -> Result<Option<Box<dyn Event>>, EvalError> {
    while let Some(Reverse((deadline, event_id))) = self.event_queue.peek() {
      if !self.pending_events.contains_key(&event_id) {
        // cancelled
        self.event_queue.pop();
        continue;
      }

      if &self.clock_monotonic()? >= deadline {
        let event = self
          .pending_events
          .remove(&event_id)
          .expect("pending_events should contain the current event");
        return Ok(Some(event));
      }
      return Ok(None);
    }
    Ok(None)
  }
}

pub struct EventWaiter {
  event_loop: Rc<RefCell<EventLoop>>,
}

impl EventWaiter {
  pub async fn wait(event_loop: Rc<RefCell<EventLoop>>) {
    EventWaiter { event_loop }.await
  }
}

impl Future for EventWaiter {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
    self
      .event_loop
      .borrow_mut()
      .waker
      .replace(cx.waker().clone());

    match self.event_loop.borrow_mut().next_wait() {
      Ok(EventWait::Ready) => Poll::Ready(()),
      Ok(EventWait::Delay(_)) => Poll::Pending,
      Ok(EventWait::Idle) => Poll::Pending,
      Err(_) => panic!("aaaah"),
    }
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
