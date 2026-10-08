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

pub enum EventAction {
  /// an event is due to be dispatched
  Ready,
  /// an event is scheduled after this delay (suspend for this long)
  WaitDuration(Duration),
  /// no events are pending; loop is idle (suspend indefinitely)
  Idle,
}

pub trait Event {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut EvalContext,
  ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>>;
}

enum EventItem {
  Event(Box<dyn Event>),
  Sleep,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Category {
  Sleep,
  Event,
}

pub struct EventLoop {
  pub(crate) clock_monotonic: Option<Rc<dyn Fn() -> Duration + 'static>>,
  event_id: EventId,
  pending_events: HashMap<EventId, EventItem>,
  event_queue: BinaryHeap<Reverse<(Category, Duration, EventId)>>,
  waker: Option<Waker>,
  pub yield_after: Duration,
  yield_deadline: Duration,
}

impl EventLoop {
  pub fn new(yield_after: Duration) -> EventLoop {
    EventLoop {
      clock_monotonic: None,
      event_id: 1,
      pending_events: HashMap::new(),
      event_queue: BinaryHeap::new(),
      waker: None,
      yield_after,
      yield_deadline: Duration::MAX,
    }
  }

  pub fn clock_monotonic(&self) -> Result<Duration, EvalError> {
    let Some(clock_monotonic) = &self.clock_monotonic else {
      return Err(EvalError::Generic(
        "EventLoop missing clock_monotonic".to_string(),
      ));
    };

    Ok(clock_monotonic())
  }

  fn maybe_wake(&self) {
    if let Some(waker) = &self.waker {
      waker.wake_by_ref();
    }
  }

  /// mark the beginning of an execution step
  pub fn start_step(&mut self) {
    self.yield_deadline = match self.clock_monotonic() {
      Ok(time) => time + self.yield_after,
      Err(_) => Duration::MAX,
    };
    self.waker = None;
  }

  /// mark the end of an execution step
  pub fn end_step(&mut self, waker: &Waker) {
    match &mut self.waker {
      Some(current) => current.clone_from(waker),
      None => self.waker = Some(waker.clone()),
    }
  }

  /// enqueue a blocking sleep marker that takes priority over events
  pub fn push_sleep(&mut self, deadline: Duration) -> EventId {
    let event_id = self.event_id;
    self.event_id += 1;

    self.pending_events.insert(event_id, EventItem::Sleep);
    self
      .event_queue
      .push(Reverse((Category::Sleep, deadline, event_id)));
    self.maybe_wake();

    event_id
  }

  /// enqueue an event to dispatch at a given deadline
  pub fn push_scheduled(&mut self, event: Box<dyn Event>, deadline: Duration) -> EventId {
    let event_id = self.event_id;
    self.event_id += 1;

    self
      .pending_events
      .insert(event_id, EventItem::Event(event));
    self
      .event_queue
      .push(Reverse((Category::Event, deadline, event_id)));
    self.maybe_wake();

    event_id
  }

  /// enqueue an event to dispatch ASAP, before any timers, but after blocking sleeps
  pub fn push_immediate(&mut self, event: Box<dyn Event>) -> EventId {
    self.push_scheduled(event, Duration::ZERO)
  }

  /// remove an event from the queue
  pub fn remove(&mut self, event_id: EventId) {
    self.pending_events.remove(&event_id);
  }

  /// get the number of queued events, ignoring other markers
  pub fn count_pending(&self) -> usize {
    self
      .pending_events
      .iter()
      .filter(|(_, item)| matches!(item, EventItem::Event(_)))
      .count()
  }

  /// get the next event, if any, without removing any events from the queue
  pub fn next_action(&mut self) -> Result<EventAction, EvalError> {
    while let Some(Reverse((_, deadline, event_id))) = self.event_queue.peek() {
      if !self.pending_events.contains_key(&event_id) {
        // cancelled or not an event
        self.event_queue.pop();
        continue;
      };

      if &self.clock_monotonic()? < deadline {
        return Ok(EventAction::WaitDuration(
          deadline.saturating_sub(self.clock_monotonic()?),
        ));
      }

      return Ok(EventAction::Ready);
    }
    Ok(EventAction::Idle)
  }

  /// take the next ready event, if any.
  pub fn take_event(&mut self) -> Result<Option<Box<dyn Event>>, EvalError> {
    while let Some(Reverse((_, deadline, event_id))) = self.event_queue.peek() {
      if !self.pending_events.contains_key(&event_id) {
        // cancelled or not an event
        self.event_queue.pop();
        continue;
      };

      if &self.clock_monotonic()? < deadline {
        return Ok(None);
      }

      let item = self
        .pending_events
        .remove(&event_id)
        .expect("pending_events should contain event_id");

      match item {
        EventItem::Event(event) => return Ok(Some(event)),
        EventItem::Sleep => continue,
      }
    }
    Ok(None)
  }
}

pub struct WaitForEvent {
  event_loop: Rc<RefCell<EventLoop>>,
  yielded_once: bool,
}

impl WaitForEvent {
  pub fn new(event_loop: Rc<RefCell<EventLoop>>) -> Self {
    WaitForEvent {
      event_loop,
      yielded_once: false,
    }
  }
}

impl Future for WaitForEvent {
  type Output = Result<Box<dyn Event>, EvalError>;

  fn poll(mut self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
    if !self.yielded_once {
      let event_loop = Rc::clone(&self.event_loop);
      if let Ok(time) = event_loop.borrow().clock_monotonic()
        && time >= event_loop.borrow().yield_deadline
      {
        self.yielded_once = true;
        return Poll::Pending;
      }
    }

    match self.event_loop.borrow_mut().take_event() {
      Ok(event) => match event {
        Some(event) => Poll::Ready(Ok(event)),
        None => Poll::Pending,
      },
      Err(err) => Poll::Ready(Err(err)),
    }
  }
}

#[cfg(all(test, not(target_family = "wasm")))]
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
