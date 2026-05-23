use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use crate::{draw::FrameInput, renderer::RendererHandle};

#[derive(Debug)]
pub struct WindowHandle {
  window:   Arc<Window>,
  renderer: RendererHandle,
}

impl WindowHandle {
  pub fn new(window: Arc<Window>, renderer: RendererHandle) -> Self {
    Self { window, renderer }
  }

  pub fn request_redraw(&self) { self.window.request_redraw(); }

  pub fn initiate_frame(&self, frame_input: FrameInput) {
    self.renderer.send_frame_input(frame_input);
  }

  pub fn handle_resize(&self, new_size: PhysicalSize<u32>) {
    self.renderer.send_resize(new_size);
    self.request_redraw();
  }

  pub fn handle_scale_factor_change(&self, new_scale_factor: f64) {
    self.renderer.send_scale_factor_change(new_scale_factor);
    self.request_redraw();
  }
}
