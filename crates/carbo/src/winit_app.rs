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

/// The app passed to the [`winit`] event loop.
///
/// [`WinitApp`] receives commands from the [`App`](crate::app::App), forwarded
/// by the [`Executor`](crate::executor::Executor) to the
/// [`EventLoopProxy`](winit::event_loop::EventLoopProxy).
///
/// It forwards all callbacks it receives to the [`App`](crate::app::App) as
/// events.
///
/// This must run on the main thread due to platform windowing restrictions.
pub struct WinitApp {
  event_tx: EventSender,
}

impl WinitApp {
  pub fn new(event_tx: EventSender) -> Self { Self { event_tx } }

  /// Processes a command (user event) sent by the
  /// [`EventLoopProxy`](winit::event_loop::EventLoopProxy).
  fn run_command(
    &mut self,
    event_loop: &ActiveEventLoop,
    command: crate::executor::EventLoopCommand,
  ) {
    match command {
      crate::executor::EventLoopCommand::BuildWindow => {
        let now = Instant::now();
        let attrs = Window::default_attributes()
          .with_title("carbo")
          .with_inner_size(LogicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        debug!("built window in {:.02}ms", now.elapsed().as_millis_f32());

        self.event_tx.event(Event::Windowing(Box::new(
          WindowingEvent::WindowBuilt(window),
        )));
      }
      crate::executor::EventLoopCommand::ExitEventLoop => {
        event_loop.exit();
      }
    }
  }
}

impl ApplicationHandler<crate::executor::EventLoopCommand> for WinitApp {
  fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .event(Event::Windowing(Box::new(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Resumed,
      ))));
  }

  fn window_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: winit::event::WindowEvent,
  ) {
    self
      .event_tx
      .event(Event::Windowing(Box::new(WindowingEvent::Window(
        window_id, event,
      ))));
  }

  fn user_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    command: crate::executor::EventLoopCommand,
  ) {
    self.run_command(event_loop, command);
  }

  fn device_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    device_id: DeviceId,
    event: winit::event::DeviceEvent,
  ) {
    self
      .event_tx
      .event(Event::Windowing(Box::new(WindowingEvent::Device(
        device_id, event,
      ))));
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .event(Event::Windowing(Box::new(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Suspended,
      ))));
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    // app loop may already have exited. avoids a potential panic.
    let _ = self.event_tx.try_event(Event::Windowing(Box::new(
      WindowingEvent::EventLoop(WinitEventLoopEvent::Exiting),
    )));
  }
}
