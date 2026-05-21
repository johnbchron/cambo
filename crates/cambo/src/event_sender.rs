use std::sync::mpsc;

use crate::app::Event;

/// Convenience trait for items that can send events.
pub trait EventSender {
  fn event_sender_handle(&self) -> &mpsc::Sender<Event>;
  fn event(&self, event: Event) {
    self
      .event_sender_handle()
      .send(event)
      .expect("failed to send event: app thread has exited");
  }
}
