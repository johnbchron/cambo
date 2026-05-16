use std::sync::{Arc, mpsc};

use winit::{
  application::ApplicationHandler, event::DeviceId,
  event_loop::ActiveEventLoop, window::WindowId,
};

use crate::{
  app::{Event, WindowCommand, WindowingEvent, WinitEventLoopEvent},
  gpu_context::GpuContext,
  window_state::WindowState,
};

pub struct WindowApp {
  pub event_tx:    mpsc::Sender<Event>,
  pub gpu_context: Arc<GpuContext>,
}

impl ApplicationHandler<WindowCommand> for WindowApp {
  fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .send(Event::Windowing(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Resumed,
      )))
      .unwrap();
  }

  fn window_event(
    &mut self,
    _event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: winit::event::WindowEvent,
  ) {
    self
      .event_tx
      .send(Event::Windowing(WindowingEvent::Window(window_id, event)))
      .unwrap();
  }

  fn user_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    command: WindowCommand,
  ) {
    match command {
      WindowCommand::BuildWindow => {
        let window_state = WindowState::new(&self.gpu_context, event_loop);
        self
          .event_tx
          .send(Event::Windowing(WindowingEvent::WindowBuilt(window_state)))
          .unwrap();
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
      .send(Event::Windowing(WindowingEvent::Device(device_id, event)))
      .unwrap();
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .send(Event::Windowing(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Suspended,
      )))
      .unwrap();
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
    self
      .event_tx
      .send(Event::Windowing(WindowingEvent::EventLoop(
        WinitEventLoopEvent::Exiting,
      )))
      .unwrap();
  }
}
