use std::sync::{Arc, mpsc};

use miette::{Context, IntoDiagnostic};
use winit::{
  dpi::PhysicalSize,
  event::{DeviceEvent, DeviceId, WindowEvent},
  event_loop::{ControlFlow, EventLoop, EventLoopProxy},
  window::{Window, WindowId},
};

use crate::{
  draw::FrameInput,
  event_sender::EventSender,
  gpu_context::GpuContext,
  renderer::{Renderer, RendererHandle},
  window_handle::WindowHandle,
  winit_app::WinitApp,
};

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
          self.drop_window_handle();
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
        Event::ExitRequested => todo!(),
        Event::CriticalFailure(report) => todo!(),
      }
    }

    Ok(())
  }

  fn accept_window_handle(&mut self, window_handle: WindowHandle) {
    window_handle.request_redraw();
    self.state.window = Some(window_handle);
  }

  fn drop_window_handle(&mut self) { self.state.window = None; }

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

#[derive(Debug)]
pub enum Command {
  EventLoopCommand(EventLoopCommand),
  SpawnRenderer(Arc<Window>, Arc<GpuContext>),
}

#[derive(Debug)]
pub enum EventLoopCommand {
  BuildWindow,
}

pub struct Executor {
  command_rx: mpsc::Receiver<Command>,
  event_tx:   mpsc::Sender<Event>,
  winit_tx:   EventLoopProxy<EventLoopCommand>,
}

impl EventSender for Executor {
  fn event_sender_handle(&self) -> &mpsc::Sender<Event> { &self.event_tx }
}

impl Executor {
  pub fn run(&mut self) -> miette::Result<()> {
    while let Ok(command) = self.command_rx.recv() {
      match command {
        Command::EventLoopCommand(event_loop_command) => {
          tracing::debug!(
            ?event_loop_command,
            "forwarding command to winit event loop"
          );
          let _ = self.winit_tx.send_event(event_loop_command);
        }
        Command::SpawnRenderer(window, gpu) => {
          tracing::debug!(
            window.id = ?window.id(),
            "spawning renderer for window"
          );
          let result =
            Renderer::launch(gpu, window.clone(), self.event_tx.clone())
              .context("failed to launch renderer");

          match result {
            Ok(handle) => {
              self.event(Event::RendererSpawned(WindowHandle::new(
                window, handle,
              )));
            }
            Err(error) => {
              self.event(Event::CriticalFailure(error));
            }
          }
        }
      }
    }

    Ok(())
  }
}
