use std::sync::Arc;

use winit::window::Window;

use crate::renderer::RendererHandle;

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

  pub fn renderer(&self) -> &RendererHandle { &self.renderer }
}
