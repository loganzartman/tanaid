# event loop refactor plan

goal: enable `bind` in tanaid-tk to receieve keyboard inputs.

```rust
pub trait Event {
  fn dispatch<'a>(
    self: Box<Self>,
    ctx: &'a mut EvalContext,
  ) -> Pin<Box<dyn Future<Output = Result<(), EvalError>> + 'a>>;
}
```

- event loop schedules then FIFOs
  - events: `HashMap<id_monotonic, Box<dyn Event>>`
  - pending: `PriorityQueue<(deadline, id_monotonic)>`
  - due: `Queue<id_monotonic>`
  - pushed events get deadline 0, to put them in front of elapsed timers; roughly matches tcl
- support pushing events to event_loop from integrators
  - waker: `Option<Waker>`
  - expose event_loop on context (as shared Rc)
- `bind` handler matches EvalError::Break and EvalError::Continue
  - break stops propagation
