use std::sync::{Arc, mpsc};

use miette::{Context, IntoDiagnostic};
use winit::{
  dpi::PhysicalSize,
  event::WindowEvent,
  event_loop::{ControlFlow, EventLoop},
};

use crate::{
  draw::FrameInput,
  event::{Event, WindowingEvent, WinitEventLoopEvent},
  executor::{Command, EventLoopCommand, Executor},
  gpu_context::GpuContext,
  renderer::RendererHandle,
  window_handle::WindowHandle,
  winit_app::WinitApp,
};

pub struct App {
  event_rx:   mpsc::Receiver<Event>,
  state:      AppState,
  command_tx: mpsc::Sender<Command>,
}

impl App {
  pub fn launch(state: AppState) -> miette::Result<()> {
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();

    let mut winit_app = WinitApp::new(event_tx.clone());
    let window_event_loop = EventLoop::<EventLoopCommand>::with_user_event()
      .build()
      .into_diagnostic()
      .context("failed to build winit event loop")?;
    window_event_loop.set_control_flow(ControlFlow::Wait);
    let winit_tx = window_event_loop.create_proxy();

    let app = App {
      event_rx,
      state,
      command_tx,
    };

    std::thread::Builder::new()
      .name("app".into())
      .spawn(move || {
        let mut app = app;
        app.run().unwrap();
      })
      .into_diagnostic()
      .context("failed to launch app thread")?;

    let executor = Executor {
      command_rx,
      winit_tx,
      event_tx,
    };

    std::thread::Builder::new()
      .name("executor".into())
      .spawn(move || {
        let mut executor = executor;
        executor.run().unwrap();
      })
      .into_diagnostic()
      .context("failed to launch executor thread")?;

    window_event_loop
      .run_app(&mut winit_app)
      .into_diagnostic()
      .context("failed to run winit event loop")?;

    Ok(())
  }

  fn command(&self, command: Command) {
    tracing::debug!(?command, "sending command");
    self.command_tx.send(command).unwrap();
  }

  pub fn run(&mut self) -> miette::Result<()> {
    while let Ok(event) = self.event_rx.recv() {
      tracing::debug!(?event, "received event");
      match event {
        Event::Windowing(WindowingEvent::EventLoop(
          WinitEventLoopEvent::Resumed,
        )) => {
          self
            .command(Command::EventLoopCommand(EventLoopCommand::BuildWindow));
        }
        Event::Windowing(WindowingEvent::EventLoop(
          WinitEventLoopEvent::Suspended,
        )) => {
          self.drop_window();
        }
        Event::Windowing(WindowingEvent::EventLoop(
          WinitEventLoopEvent::Exiting,
        )) => {
          tracing::info!("winit event loop is exiting => ending app loop");
          break;
        }

        Event::Windowing(WindowingEvent::Window(
          _,
          WindowEvent::Resized(new_size),
        )) => {
          self.affect_resize(new_size);
        }
        Event::Windowing(WindowingEvent::Window(
          _,
          WindowEvent::ScaleFactorChanged { scale_factor, .. },
        )) => {
          self.affect_scale_factor_change(scale_factor);
        }
        Event::Windowing(WindowingEvent::Window(
          _,
          WindowEvent::RedrawRequested,
        )) => {
          self.initiate_frame();
        }
        Event::Windowing(WindowingEvent::Window(
          _,
          WindowEvent::CloseRequested,
        )) => {
          self.shut_down_app();
          return Ok(());
        }

        Event::Windowing(WindowingEvent::Window(w_id, _window_event)) => {
          tracing::debug!(window.id = ?w_id, "ignoring unimplemented window event");
          if let Some(wh) = self.get_window_handle() {
            wh.request_redraw();
          }
        }
        Event::Windowing(WindowingEvent::Device(d_id, _device_event)) => {
          tracing::debug!(device.id = ?d_id, "ignoring unimplemented device event");
        }

        Event::Windowing(WindowingEvent::WindowBuilt(window)) => {
          self.command(Command::SpawnRenderer(window, self.state.gpu.clone()));
        }
        Event::RendererSpawned(window_handle) => {
          self.accept_window_handle(window_handle);
        }
        Event::ExitRequested => {
          self.shut_down_app();
          return Ok(());
        }
        Event::CriticalFailure(report) => {
          self.shut_down_app();
          return Err(report);
        }
      }
    }

    Ok(())
  }

  fn accept_window_handle(&mut self, window_handle: WindowHandle) {
    window_handle.request_redraw();
    self.state.window = Some(window_handle);
  }

  fn drop_window(&mut self) { self.state.window = None; }

  fn shut_down_app(&mut self) {
    self.drop_window();
    self.command(Command::EventLoopCommand(EventLoopCommand::ExitEventLoop));
  }

  fn initiate_frame(&self) {
    let frame_input = FrameInput {};

    let Some(renderer) = self.get_renderer() else {
      tracing::warn!("attempted to initiate a frame without a window present");
      return;
    };

    renderer.send_frame_input(frame_input);
  }

  fn affect_resize(&self, new_size: PhysicalSize<u32>) {
    let Some(window_handle) = self.get_window_handle() else {
      tracing::warn!("attempted to affect a resize without a window present");
      return;
    };

    window_handle.renderer().send_resize(new_size);
    window_handle.request_redraw();
  }

  fn affect_scale_factor_change(&self, new_scale_factor: f64) {
    let Some(window_handle) = self.get_window_handle() else {
      tracing::warn!(
        "attempted to affect a scale factor change without a window present"
      );
      return;
    };

    window_handle
      .renderer()
      .send_scale_factor_change(new_scale_factor);
    window_handle.request_redraw();
  }

  fn get_window_handle(&self) -> Option<&WindowHandle> {
    self.state.window.as_ref()
  }

  fn get_renderer(&self) -> Option<&RendererHandle> {
    self.get_window_handle().map(|wh| wh.renderer())
  }
}

pub struct AppState {
  gpu:    Arc<GpuContext>,
  window: Option<WindowHandle>,
}

impl AppState {
  pub fn build() -> miette::Result<Self> {
    Ok(AppState {
      gpu:    Arc::new(
        GpuContext::new().context("failed to build GPU context")?,
      ),
      window: None,
    })
  }
}
