use std::sync::mpsc::{self, SendError};

use crate::event::Event;

#[derive(Clone, Debug)]
pub struct EventSender {
  event_tx: mpsc::Sender<Event>,
}

impl EventSender {
  pub fn new(event_tx: mpsc::Sender<Event>) -> Self { Self { event_tx } }

  pub fn event(&self, event: Event) {
    self
      .event_tx
      .send(event)
      .expect("failed to send event: app thread has exited");
  }

  pub fn try_event(&self, event: Event) -> Result<(), SendError<Event>> {
    self.event_tx.send(event)
  }
}
