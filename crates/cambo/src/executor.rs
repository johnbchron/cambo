use std::sync::{Arc, mpsc};

use miette::Context;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{
  event::Event, event_sender::EventSender, gpu_context::GpuContext,
  renderer::Renderer, window_handle::WindowHandle,
};

#[derive(Debug)]
pub enum Command {
  EventLoopCommand(EventLoopCommand),
  SpawnRenderer(Arc<Window>, Arc<GpuContext>),
}

#[derive(Debug)]
pub enum EventLoopCommand {
  BuildWindow,
  ExitEventLoop,
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
  pub fn new(
    command_rx: mpsc::Receiver<Command>,
    event_tx: mpsc::Sender<Event>,
    winit_tx: EventLoopProxy<EventLoopCommand>,
  ) -> Self {
    Self {
      command_rx,
      event_tx,
      winit_tx,
    }
  }

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
