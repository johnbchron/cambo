use vello::{
  Scene,
  kurbo::{Affine, Circle, Point, RoundedRect, Vec2},
  peniko::{Brush, Fill, color::palette},
};
use wgpu::{
  CommandEncoderDescriptor, TextureDescriptor, TextureDimension, TextureUsages,
  TextureViewDescriptor,
};
use winit::{
  application::ApplicationHandler,
  dpi::PhysicalPosition,
  event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
  event_loop::ActiveEventLoop,
  keyboard::{Key, NamedKey},
  window::WindowId,
};

use crate::{
  gpu_context::GpuContext,
  window_state::{InputState, WindowState},
};

pub struct App {
  pub gpu:    GpuContext,
  pub window: Option<WindowState>,
}

impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    if self.window.is_some() {
      return;
    }
    let ws = WindowState::new(&self.gpu, event_loop);
    ws.request_redraw();
    self.window = Some(ws);
  }

  fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
    // Only the window and surface are dropped; GPU context survives.
    self.window = None;
  }

  fn exiting(&mut self, _event_loop: &ActiveEventLoop) { self.window = None; }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _id: WindowId,
    event: WindowEvent,
  ) {
    let Some(ws) = self.window.as_mut() else {
      return;
    };

    match event {
      // ── lifecycle / surface ─────────────────────────────
      WindowEvent::CloseRequested => event_loop.exit(),

      WindowEvent::Resized(new_size) => {
        ws.surface
          .resize(&self.gpu.device, new_size.width, new_size.height);
        ws.request_redraw();
      }

      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        ws.scale_factor = scale_factor;
      }

      WindowEvent::Occluded(occluded) => {
        ws.occluded = occluded;
        if !occluded {
          ws.request_redraw();
        }
      }

      WindowEvent::Focused(focused) => {
        ws.focused = focused;
      }

      // ── keyboard ────────────────────────────────────────
      WindowEvent::KeyboardInput {
        event:
          KeyEvent {
            logical_key,
            state: ElementState::Pressed,
            repeat: false,
            ..
          },
        ..
      } => {
        if let Key::Named(NamedKey::Escape) = logical_key.as_ref() {
          event_loop.exit()
        }
      }

      // ── mouse / pointer ─────────────────────────────────
      WindowEvent::CursorMoved {
        position: PhysicalPosition { x, y },
        ..
      } => {
        ws.input.cursor = Point::new(x / ws.scale_factor, y / ws.scale_factor);
        ws.request_redraw();
      }

      WindowEvent::MouseInput { state, button, .. } => {
        if button == MouseButton::Left {
          ws.input.mouse_down = state == ElementState::Pressed;
          ws.request_redraw();
        }
      }

      WindowEvent::MouseWheel { delta, .. } => {
        let (dx, dy) = match delta {
          MouseScrollDelta::LineDelta(x, y) => {
            (x as f64 * 20.0, y as f64 * 20.0)
          }
          MouseScrollDelta::PixelDelta(p) => (p.x, p.y),
        };
        ws.input.scroll_delta += Vec2::new(dx, dy);
        ws.request_redraw();
      }

      // ── drawing ─────────────────────────────────────────
      WindowEvent::RedrawRequested => {
        if ws.occluded {
          return;
        }
        render(&mut self.gpu, ws);
        ws.frame = ws.frame.wrapping_add(1);
        ws.request_redraw();
      }

      _ => {}
    }
  }
}

fn render(gpu: &mut GpuContext, ws: &mut WindowState) {
  let width = ws.surface.config.width;
  let height = ws.surface.config.height;

  let mut scene = Scene::new();
  build_scene(
    &mut scene,
    width,
    height,
    ws.scale_factor,
    &ws.input,
    ws.frame,
  );

  let surface_tex = match ws.surface.surface.get_current_texture() {
    Ok(t) => t,
    Err(wgpu::SurfaceError::Timeout) => return,
    Err(wgpu::SurfaceError::OutOfMemory) => panic!("GPU OOM"),
    Err(_) => {
      ws.surface.reconfigure(&gpu.device);
      ws.request_redraw();
      return;
    }
  };

  let target_tex = gpu.device.create_texture(&TextureDescriptor {
    label:           Some("vello target"),
    size:            wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count:    1,
    dimension:       TextureDimension::D2,
    format:          ws.surface.config.format,
    usage:           TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
    view_formats:    &[],
  });
  let target_view = target_tex.create_view(&TextureViewDescriptor::default());

  gpu
    .renderer
    .render_to_texture(
      &gpu.device,
      &gpu.queue,
      &scene,
      &target_view,
      &vello::RenderParams {
        base_color: palette::css::BLACK,
        width,
        height,
        antialiasing_method: vello::AaConfig::Area,
      },
    )
    .expect("vello render failed");

  let mut encoder = gpu
    .device
    .create_command_encoder(&CommandEncoderDescriptor::default());
  encoder.copy_texture_to_texture(
    target_tex.as_image_copy(),
    surface_tex.texture.as_image_copy(),
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
  gpu.queue.submit([encoder.finish()]);
  surface_tex.present();
}

fn build_scene(
  scene: &mut Scene,
  width: u32,
  height: u32,
  scale_factor: f64,
  input: &InputState,
  frame: u64,
) {
  let w = width as f64 / scale_factor;
  let h = height as f64 / scale_factor;
  let zoom = Affine::scale(scale_factor);

  let rect = RoundedRect::new(20.0, 20.0, w - 20.0, h - 20.0, 16.0);
  scene.fill(
    Fill::NonZero,
    zoom,
    &Brush::Solid(palette::css::DARK_SLATE_GRAY),
    None,
    &rect,
  );

  let t = frame as f64 * 0.02;
  let cx = w / 2.0 + t.cos() * 120.0;
  let cy = h / 2.0 + t.sin() * 120.0;
  let radius = if input.mouse_down { 60.0 } else { 40.0 };
  scene.fill(
    Fill::NonZero,
    zoom,
    &Brush::Solid(palette::css::CORAL),
    None,
    &Circle::new((cx, cy), radius),
  );

  scene.fill(
    Fill::NonZero,
    zoom,
    &Brush::Solid(palette::css::WHITE),
    None,
    &Circle::new(input.cursor, 16.0),
  );
}
