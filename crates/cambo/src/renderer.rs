use std::{
  sync::{Arc, mpsc},
  thread::JoinHandle,
};

use miette::{Context, IntoDiagnostic};
use vello::peniko::color::palette;
use wgpu::{
  CommandEncoderDescriptor, TextureDescriptor, TextureDimension, TextureUsages,
  TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
  draw::{FrameInput, FullFrameInput},
  event_sender::EventSender,
  gpu_context::GpuContext,
  surface_state::SurfaceState,
};

pub struct Renderer {
  gpu:                  Arc<GpuContext>,
  renderer:             vello::Renderer,
  surface_state:        SurfaceState,
  current_scale_factor: f64,
  current_frame_count:  u64,
  renderer_command_rx:  mpsc::Receiver<RendererCommand>,
  event_tx:             EventSender,
}

pub enum RendererCommand {
  FrameInput(FrameInput),
  ChangedScaleFactor(f64),
  Resized(u32, u32),
}

impl Renderer {
  pub fn launch(
    gpu: Arc<GpuContext>,
    window: Arc<Window>,
    event_tx: EventSender,
  ) -> miette::Result<RendererHandle> {
    let (renderer_command_tx, renderer_command_rx) = mpsc::channel();

    let current_scale_factor = window.scale_factor();
    let surface_state = SurfaceState::new(&gpu, window);

    let renderer = vello::Renderer::new(gpu.device(), vello::RendererOptions {
      use_cpu:              false,
      antialiasing_support: vello::AaSupport::area_only(),
      num_init_threads:     None,
      pipeline_cache:       None,
    })
    .into_diagnostic()
    .context("failed to create vello renderer")?;

    let renderer = Renderer {
      gpu,
      renderer,
      surface_state,
      current_scale_factor,
      current_frame_count: 0,
      renderer_command_rx,
      event_tx,
    };

    let join_handle = std::thread::Builder::new()
      .name("renderer".into())
      .spawn(move || {
        let mut renderer = renderer;
        renderer.run().unwrap();
      })
      .into_diagnostic()
      .context("failed to launch renderer thread")?;

    let handle = RendererHandle {
      join_handle,
      renderer_command_tx,
    };

    Ok(handle)
  }

  fn run(&mut self) -> miette::Result<()> {
    while let Ok(command) = self.renderer_command_rx.recv() {
      match command {
        RendererCommand::FrameInput(frame_input) => {
          let width = self.surface_state.surface_config.width;
          let height = self.surface_state.surface_config.height;

          let full_frame_input = FullFrameInput::new(
            frame_input,
            (width, height),
            self.current_scale_factor,
            self.current_frame_count,
          );

          let scene = full_frame_input.draw();

          let surface_tex =
            self.surface_state.surface.get_current_texture().unwrap();

          let target_tex =
            self.gpu.device().create_texture(&TextureDescriptor {
              label:           Some("vello target"),
              size:            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
              },
              mip_level_count: 1,
              sample_count:    1,
              dimension:       TextureDimension::D2,
              format:          self.surface_state.surface_config.format,
              usage:           TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_SRC,
              view_formats:    &[],
            });
          let target_view =
            target_tex.create_view(&TextureViewDescriptor::default());

          self
            .renderer
            .render_to_texture(
              self.gpu.device(),
              self.gpu.queue(),
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

          let mut encoder = self
            .gpu
            .device()
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
          self.gpu.queue().submit([encoder.finish()]);
          surface_tex.present();

          self.current_frame_count += 1;
        }
        RendererCommand::ChangedScaleFactor(new_scale_factor) => {
          self.current_scale_factor = new_scale_factor;
        }
        RendererCommand::Resized(physical_width, physical_height) => {
          self.surface_state.resize_surface(
            self.gpu.device(),
            physical_width,
            physical_height,
          );
        }
      }
    }

    Ok(())
  }
}

#[derive(Debug)]
pub struct RendererHandle {
  join_handle:         JoinHandle<()>,
  renderer_command_tx: mpsc::Sender<RendererCommand>,
}

impl RendererHandle {
  pub fn send_frame_input(&self, input: FrameInput) {
    self
      .renderer_command_tx
      .send(RendererCommand::FrameInput(input))
      .unwrap();
  }

  pub fn send_resize(&self, new_size: PhysicalSize<u32>) {
    self
      .renderer_command_tx
      .send(RendererCommand::Resized(new_size.width, new_size.height))
      .unwrap();
  }

  pub fn send_scale_factor_change(&self, new_scale_factor: f64) {
    self
      .renderer_command_tx
      .send(RendererCommand::ChangedScaleFactor(new_scale_factor))
      .unwrap();
  }
}
