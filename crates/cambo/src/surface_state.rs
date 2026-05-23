use std::sync::Arc;

use wgpu::{SurfaceConfiguration, TextureUsages};
use winit::window::Window;

use crate::gpu_context::GpuContext;

/// Manages a [`wgpu::Surface`]. This is held by the
/// [`Renderer`](crate::renderer::Renderer) and is used to present frames to the
/// window.
#[derive(Debug)]
pub struct SurfaceState {
  pub surface:        wgpu::Surface<'static>,
  pub surface_config: SurfaceConfiguration,
}

impl SurfaceState {
  /// Constructs a surface with its config, given the [`GpuContext`] and target
  /// [`Window`].
  pub fn new(gpu: &GpuContext, window: Arc<Window>) -> Self {
    let size = window.inner_size();
    let surface = gpu.instance().create_surface(window).unwrap();

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
      surface,
      surface_config,
    }
  }

  /// Resizes and reconfigures the surface.
  pub fn resize_surface(
    &mut self,
    device: &wgpu::Device,
    width: u32,
    height: u32,
  ) {
    self.surface_config.width = width.max(1);
    self.surface_config.height = height.max(1);
    self.reconfigure_surface(device);
  }

  /// Reconfigures the surface with the current config.
  pub fn reconfigure_surface(&self, device: &wgpu::Device) {
    self.surface.configure(device, &self.surface_config);
  }
}
