use std::sync::Arc;

use winit::{
  event::{DeviceEvent, DeviceId, WindowEvent},
  window::{Window, WindowId},
};

use crate::window_handle::WindowHandle;

#[derive(Debug)]
pub enum WindowingEvent {
  EventLoop(WinitEventLoopEvent),
  Window(WindowId, WindowEvent),
  Device(DeviceId, DeviceEvent),
  WindowBuilt(Arc<Window>),
}

#[derive(Debug)]
pub enum WinitEventLoopEvent {
  Resumed,
  Suspended,
  Exiting,
}

#[derive(Debug)]
pub enum Event {
  Windowing(WindowingEvent),
  RendererSpawned(WindowHandle),
  ExitRequested,
  CriticalFailure(miette::Report),
}
