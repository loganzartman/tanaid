use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;
use std::time::Duration;
use std::{cell::RefCell, task::Poll};

use crate::eval_error::EvalError;
use crate::event_loop::{EventId, EventLoop};

pub struct Sleep {
  event_loop: Rc<RefCell<EventLoop>>,
  event_id: EventId,
  deadline: Duration,
}

impl Sleep {
  pub fn new(event_loop: Rc<RefCell<EventLoop>>, event_id: EventId, deadline: Duration) -> Self {
    Sleep {
      event_loop,
      event_id,
      deadline,
    }
  }
}

impl Future for Sleep {
  type Output = Result<(), EvalError>;

  fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
    match self.event_loop.borrow().clock_monotonic() {
      Ok(now) => {
        if now >= self.deadline {
          Poll::Ready(Ok(()))
        } else {
          Poll::Pending
        }
      }
      Err(e) => Poll::Ready(Err(e)),
    }
  }
}

impl Drop for Sleep {
  fn drop(&mut self) {
    self.event_loop.borrow_mut().remove(self.event_id);
  }
}
