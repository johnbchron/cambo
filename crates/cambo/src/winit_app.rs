use std::sync::{Arc, mpsc};

use winit::{
  application::ApplicationHandler,
  dpi::LogicalSize,
  event::DeviceId,
  event_loop::ActiveEventLoop,
  window::{Window, WindowId},
};

use crate::{
  app::{Event, EventLoopCommand, WindowingEvent, WinitEventLoopEvent},
  event_sender::EventSender,
};

pub struct WinitApp {
  event_tx: mpsc::Sender<Event>,
}

impl WinitApp {
  pub fn new(event_tx: mpsc::Sender<Event>) -> Self { Self { event_tx } }
}

impl EventSender for WinitApp {
  fn event_sender_handle(&self) -> &mpsc::Sender<Event> { &self.event_tx }
}

impl ApplicationHandler<EventLoopCommand> for WinitApp {
  fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    self.event(Event::Windowing(WindowingEvent::EventLoop(
      WinitEventLoopEvent::Resumed,
    )));
  }

  fn window_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: winit::event::WindowEvent,
  ) {
    self.event(Event::Windowing(WindowingEvent::Window(window_id, event)));
  }

  fn user_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    command: EventLoopCommand,
  ) {
    match command {
      EventLoopCommand::BuildWindow => {
        let attrs = Window::default_attributes()
          .with_title("cambo")
          .with_inner_size(LogicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        self.event(Event::Windowing(WindowingEvent::WindowBuilt(window)));
      }
      EventLoopCommand::ExitEventLoop => {
        event_loop.exit();
      }
    }
  }

  fn device_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    device_id: DeviceId,
    event: winit::event::DeviceEvent,
  ) {
    self.event(Event::Windowing(WindowingEvent::Device(device_id, event)));
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    self.event(Event::Windowing(WindowingEvent::EventLoop(
      WinitEventLoopEvent::Suspended,
    )));
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    // app loop may already have exited. avoids a potential panic.
    let _ = self.try_event(Event::Windowing(WindowingEvent::EventLoop(
      WinitEventLoopEvent::Exiting,
    )));
  }
}
