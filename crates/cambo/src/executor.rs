use std::{
  sync::{Arc, mpsc},
  time::Instant,
};

use miette::Context;
use tracing::debug;
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
  event_tx:   EventSender,
  winit_tx:   EventLoopProxy<EventLoopCommand>,
}

impl Executor {
  pub fn new(
    command_rx: mpsc::Receiver<Command>,
    event_tx: EventSender,
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
          let now = Instant::now();
          let result =
            Renderer::launch(gpu, window.clone(), self.event_tx.clone())
              .context("failed to launch renderer");
          debug!(
            "launched renderer in {:.2}ms",
            now.elapsed().as_millis_f32()
          );

          match result {
            Ok(handle) => {
              self
                .event_tx
                .event(Event::RendererSpawned(WindowHandle::new(
                  window, handle,
                )));
            }
            Err(error) => {
              self.event_tx.event(Event::CriticalFailure(error));
            }
          }
        }
      }
    }

    Ok(())
  }
}
