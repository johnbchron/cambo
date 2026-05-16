use std::sync::{Arc, mpsc};

use miette::{Context, IntoDiagnostic};
use winit::{
  event::{DeviceEvent, DeviceId, WindowEvent},
  event_loop::{ControlFlow, EventLoop, EventLoopProxy},
  window::WindowId,
};

use crate::{
  gpu_context::GpuContext, window_app::WindowApp, window_state::WindowState,
};

#[derive(Debug)]
pub enum WindowingEvent {
  EventLoop(WinitEventLoopEvent),
  Window(WindowId, WindowEvent),
  Device(DeviceId, DeviceEvent),
  WindowBuilt(WindowState),
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
  ExitRequested,
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

    let mut window_app = WindowApp {
      event_tx,
      gpu_context: state.gpu.clone(),
    };
    let window_event_loop = EventLoop::<WindowCommand>::with_user_event()
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

    std::thread::spawn(move || {
      let mut app = app;
      app.run().unwrap();
    });

    let executor = Executor {
      command_rx,
      winit_tx,
    };

    std::thread::spawn(move || {
      let mut executor = executor;
      executor.run().unwrap();
    });

    window_event_loop
      .run_app(&mut window_app)
      .into_diagnostic()
      .context("failed to run winit event loop")?;

    Ok(())
  }

  fn send(&self, command: Command) {
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
          self.send(Command::WindowCommand(WindowCommand::BuildWindow));
        }
        _ => (),
      }
    }

    Ok(())
  }
}

pub struct AppState {
  gpu: Arc<GpuContext>,
}

impl AppState {
  pub fn build() -> miette::Result<Self> {
    Ok(AppState {
      gpu: Arc::new(GpuContext::new().context("failed to build GPU context")?),
    })
  }
}

#[derive(Debug)]
pub enum Command {
  WindowCommand(WindowCommand),
}

#[derive(Debug)]
pub enum WindowCommand {
  BuildWindow,
}

pub struct Executor {
  command_rx: mpsc::Receiver<Command>,
  winit_tx:   EventLoopProxy<WindowCommand>,
}

impl Executor {
  pub fn run(&mut self) -> miette::Result<()> {
    while let Ok(command) = self.command_rx.recv() {
      match command {
        Command::WindowCommand(window_command) => {
          tracing::debug!(
            ?window_command,
            "forwarding window command to winit event loop"
          );
          let _ = self.winit_tx.send_event(window_command);
        }
      }
    }

    Ok(())
  }
}
