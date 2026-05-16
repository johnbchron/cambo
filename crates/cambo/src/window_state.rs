use std::sync::Arc;

use wgpu::{SurfaceConfiguration, TextureUsages};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

use crate::gpu_context::GpuContext;

#[derive(Debug)]
pub struct WindowState {
  pub window:         Arc<Window>,
  pub surface:        wgpu::Surface<'static>,
  pub surface_config: SurfaceConfiguration,
}

impl WindowState {
  pub fn new(gpu: &GpuContext, event_loop: &ActiveEventLoop) -> Self {
    let attrs = Window::default_attributes()
      .with_title("cambo")
      .with_inner_size(LogicalSize::new(800, 600));
    let window = Arc::new(event_loop.create_window(attrs).unwrap());

    let surface = gpu.instance().create_surface(window.clone()).unwrap();

    let size = window.inner_size();
    let surface_config = SurfaceConfiguration {
      usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
      format: wgpu::TextureFormat::Rgba8Unorm,
      width: size.width.max(1),
      height: size.height.max(1),
      present_mode: wgpu::PresentMode::AutoVsync,
      desired_maximum_frame_latency: 2,
      alpha_mode: wgpu::CompositeAlphaMode::Auto,
      view_formats: vec![],
    };

    surface.configure(gpu.device(), &surface_config);

    Self {
      window,
      surface,
      surface_config,
    }
  }

  pub fn request_redraw(&self) { self.window.request_redraw(); }

  pub fn resize_surface(
    &mut self,
    device: &wgpu::Device,
    width: u32,
    height: u32,
  ) {
    self.surface_config.width = width.max(1);
    self.surface_config.height = height.max(1);
    self.surface.configure(device, &self.surface_config);
  }

  pub fn reconfigure_surface(&self, device: &wgpu::Device) {
    self.surface.configure(device, &self.surface_config);
  }
}
