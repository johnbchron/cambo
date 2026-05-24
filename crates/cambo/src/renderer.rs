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

/// The [`Renderer`] lives in its own thread, and is responsible for:
/// - Holding the surface and [`vello::Renderer`].
/// - Receiving resizing events and reconfiguring the surface when needed.
/// - Receiving [`FrameInput`]s, turning them into [`FullFrameInput`]s, drawing
///   them to a [`vello::Scene`], and then rendering them to the surface.
///
/// Interactions with the [`Renderer`] happen through the [`RendererHandle`].
/// There is only one [`RendererHandle`] per [`Renderer`], and it sends
/// [`RendererCommand`]s and controls the lifecycle of the [`Renderer`]. When
/// the [`RendererHandle`] drops, the [`Renderer`]'s thread ends and it drops.
/// The [`RendererHandle`] can send new [`FrameInput`]s to be rendered and
/// resizing and scale notifications.
///
/// To turn a [`FrameInput`] into a [`FullFrameInput`], we need the physical
/// size of the surface we're drawing to, the scale factor we're drawing at, and
/// the current frame count. We can keep the physical size stored in the
/// surface, but we have to keep track of the scale factor and frame count as
/// mutable state in the [`Renderer`].
pub struct Renderer {
  gpu:                  Arc<GpuContext>,
  renderer:             vello::Renderer,
  surface_state:        SurfaceState,
  current_scale_factor: f64,
  current_frame_count:  u64,
  renderer_command_rx:  mpsc::Receiver<RendererCommand>,
  _event_tx:            EventSender,
}

/// Sent from the [`RendererHandle`] to the [`Renderer`].
enum RendererCommand {
  FrameInput(FrameInput),
  ChangedScaleFactor(f64),
  Resized(u32, u32),
}

impl Renderer {
  /// Builds the [`Renderer`], starts it in its own thread, and returns a
  /// [`RendererHandle`].
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
      _event_tx: event_tx,
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
      _join_handle: join_handle,
      renderer_command_tx,
    };

    Ok(handle)
  }

  /// Runs the [`Renderer`] event loop.
  fn run(&mut self) -> miette::Result<()> {
    while let Ok(command) = self.renderer_command_rx.recv() {
      match command {
        RendererCommand::FrameInput(frame_input) => {
          let width = self.surface_state.config_width();
          let height = self.surface_state.config_height();

          let full_frame_input = FullFrameInput::new(
            frame_input,
            (width, height),
            self.current_scale_factor,
            self.current_frame_count,
          );

          let scene = full_frame_input.draw();

          let surface_tex = self.surface_state.get_current_texture();

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
              format:          self.surface_state.config_format(),
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

/// The handle returned by [`Renderer::launch`]. This is the only way to
/// interact with the [`Renderer`], and dropping it will stop the [`Renderer`]
/// after it finishes the work at hand.
#[derive(Debug)]
pub struct RendererHandle {
  _join_handle:        JoinHandle<()>,
  renderer_command_tx: mpsc::Sender<RendererCommand>,
}

impl RendererHandle {
  /// Sends a [`FrameInput`] to the renderer, to be drawn and rendered to the
  /// [`Renderer`]'s surface.
  pub fn send_frame_input(&self, input: FrameInput) {
    self
      .renderer_command_tx
      .send(RendererCommand::FrameInput(input))
      .unwrap();
  }

  /// Notifies the [`Renderer`] of a resize event, and prompts it to reconfigure
  /// its surface.
  pub fn send_resize(&self, new_size: PhysicalSize<u32>) {
    self
      .renderer_command_tx
      .send(RendererCommand::Resized(new_size.width, new_size.height))
      .unwrap();
  }

  /// Notifies the [`Renderer`] of a scale factor change, prompting it to render
  /// later frames at this scale factor.
  pub fn send_scale_factor_change(&self, new_scale_factor: f64) {
    self
      .renderer_command_tx
      .send(RendererCommand::ChangedScaleFactor(new_scale_factor))
      .unwrap();
  }
}
