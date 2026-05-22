use std::{sync::Arc, time::Instant};

use tracing::debug;
use winit::{
  application::ApplicationHandler,
  dpi::LogicalSize,
  event::DeviceId,
  event_loop::ActiveEventLoop,
  window::{Window, WindowId},
};

use crate::{
  event::{Event, WindowingEvent, WinitEventLoopEvent},
  event_sender::EventSender,
};

pub struct WinitApp {
  event_tx: EventSender,
}

impl WinitApp {
  pub fn new(event_tx: EventSender) -> Self { Self { event_tx } }
}

impl ApplicationHandler<crate::executor::EventLoopCommand> for WinitApp {
  fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .event(Event::Windowing(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Resumed,
      )));
  }

  fn window_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: winit::event::WindowEvent,
  ) {
    self
      .event_tx
      .event(Event::Windowing(WindowingEvent::Window(window_id, event)));
  }

  fn user_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    command: crate::executor::EventLoopCommand,
  ) {
    match command {
      crate::executor::EventLoopCommand::BuildWindow => {
        let now = Instant::now();
        let attrs = Window::default_attributes()
          .with_title("cambo")
          .with_inner_size(LogicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        debug!("built window in {:.02}ms", now.elapsed().as_millis_f32());

        self
          .event_tx
          .event(Event::Windowing(WindowingEvent::WindowBuilt(window)));
      }
      crate::executor::EventLoopCommand::ExitEventLoop => {
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
    self
      .event_tx
      .event(Event::Windowing(WindowingEvent::Device(device_id, event)));
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .event(Event::Windowing(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Suspended,
      )));
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    // app loop may already have exited. avoids a potential panic.
    let _ =
      self
        .event_tx
        .try_event(Event::Windowing(WindowingEvent::EventLoop(
          WinitEventLoopEvent::Exiting,
        )));
  }
}
