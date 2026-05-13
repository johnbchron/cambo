use std::sync::Arc;

use vello::kurbo::{Point, Vec2};
use wgpu::{SurfaceConfiguration, TextureUsages};
use winit::{dpi::LogicalSize, event_loop::ActiveEventLoop, window::Window};

use crate::gpu_context::GpuContext;

pub struct WindowState {
  /// OS window handle.
  pub window:       Arc<Window>,
  /// Swapchain surface and its current configuration.
  pub surface:      SurfaceState,
  /// Accumulated pointer/keyboard state, read by the scene each frame.
  pub input:        InputState,
  /// True when the compositor says the window is fully hidden; skips
  /// rendering.
  pub occluded:     bool,
  /// True when the window has keyboard focus; useful for throttling.
  pub focused:      bool,
  /// Logical-to-physical ratio, updated on ScaleFactorChanged.
  pub scale_factor: f64,
  /// Monotonic frame counter, used as a time source for animation.
  pub frame:        u64,
}

pub struct SurfaceState {
  /// Swapchain surface tied to the window.
  pub surface: wgpu::Surface<'static>,
  /// Current dimensions, format, and present mode of the surface.
  pub config:  SurfaceConfiguration,
}

#[derive(Default)]
pub struct InputState {
  /// Current cursor position in logical coordinates.
  pub cursor:       Point,
  /// True while the left mouse button is held.
  pub mouse_down:   bool,
  /// Scroll delta accumulated since the last frame; zeroed after each redraw.
  pub scroll_delta: Vec2,
}

impl WindowState {
  pub fn new(gpu: &GpuContext, event_loop: &ActiveEventLoop) -> Self {
    let attrs = Window::default_attributes()
      .with_title("vello + wgpu + winit")
      .with_inner_size(LogicalSize::new(800, 600));
    let window = Arc::new(event_loop.create_window(attrs).unwrap());
    let scale_factor = window.scale_factor();
    let surface = SurfaceState::new(gpu, &window);

    Self {
      window,
      surface,
      input: Default::default(),
      occluded: false,
      focused: true,
      scale_factor,
      frame: 0,
    }
  }

  pub fn request_redraw(&self) { self.window.request_redraw(); }
}

impl SurfaceState {
  fn new(gpu: &GpuContext, window: &Arc<Window>) -> Self {
    let surface = gpu.instance.create_surface(window.clone()).unwrap();

    let size = window.inner_size();
    let config = SurfaceConfiguration {
      usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST,
      format: wgpu::TextureFormat::Rgba8Unorm,
      width: size.width.max(1),
      height: size.height.max(1),
      present_mode: wgpu::PresentMode::AutoVsync,
      desired_maximum_frame_latency: 2,
      alpha_mode: wgpu::CompositeAlphaMode::Auto,
      view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);

    Self { surface, config }
  }

  pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
    self.config.width = width.max(1);
    self.config.height = height.max(1);
    self.surface.configure(device, &self.config);
  }

  pub fn reconfigure(&self, device: &wgpu::Device) {
    self.surface.configure(device, &self.config);
  }
}
