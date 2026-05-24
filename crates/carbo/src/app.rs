use std::{
  sync::{Arc, mpsc},
  time::{Duration, Instant},
};

use miette::{Context, IntoDiagnostic};
use tracing::{debug, info, warn};
use winit::{
  dpi::PhysicalSize,
  event::WindowEvent,
  event_loop::{ControlFlow, EventLoop},
};

use crate::{
  draw::FrameInput,
  event::{Event, WindowingEvent, WinitEventLoopEvent},
  event_sender::EventSender,
  executor::{Command, EventLoopCommand, Executor},
  gpu_context::GpuContext,
  window_handle::WindowHandle,
  winit_app::WinitApp,
};

/// The fundamental decision-maker and state-holder.
///
/// All events come into [`App`]. In response to an event, the app may:
/// - Mutate something in [`AppState`]
/// - Send a command to the [`Executor`]
/// - Kick a frame off to a [`WindowHandle`]
///
/// The [`App`] is meant to run a really tight loop because all input and events
/// flow through it. If processing the event takes too long and the watchdog
/// timer is tripped, a warning log will fire.
///
/// If the thing you want to do takes more than just a quick state mutation,
/// turn it into a command. Package whatever state you need and send it to the
/// [`Executor`], and then fire an event when it's done. If you need, you can
/// receive that event and kick off another command or event. Events and
/// commands can be chained easily. You can package a state machine that you
/// pass back and forth if you wish.
///
/// ## Flows
/// ### Starting the main window
/// - [`WinitEventLoopEvent::Resumed`] is received, meaning the [`WinitApp`] got
///   it's `resumed` method called.
/// - The [`App`] sends the [`EventLoopCommand::BuildWindow`] command, which the
///   [`Executor`] forwards to the [`WinitApp`].
/// - The [`WinitApp`] builds the window, and sends it back as the
///   [`WindowingEvent::WindowBuilt`].
pub struct App {
  event_rx:   mpsc::Receiver<Event>,
  state:      AppState,
  command_tx: mpsc::Sender<Command>,
}

impl App {
  /// Builds the [`App`] and all the things it's connected to, and sets it all
  /// in motion.
  pub fn launch(state: AppState) -> miette::Result<()> {
    // build the channels
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let event_tx = EventSender::new(event_tx);

    // build the winit app
    let mut winit_app = WinitApp::new(event_tx.clone());
    let window_event_loop = EventLoop::<EventLoopCommand>::with_user_event()
      .build()
      .into_diagnostic()
      .context("failed to build winit event loop")?;
    window_event_loop.set_control_flow(ControlFlow::Wait);
    // the event loop proxy goes into the executor
    let winit_tx = window_event_loop.create_proxy();

    // build the main app
    let app = App {
      event_rx,
      state,
      command_tx,
    };

    // launch the app thread
    std::thread::Builder::new()
      .name("app".into())
      .spawn(move || {
        let mut app = app;
        app.run().unwrap();
      })
      .into_diagnostic()
      .context("failed to launch app thread")?;

    // build the executor
    let executor = Executor::new(command_rx, event_tx, winit_tx);

    // launch the executor thread
    std::thread::Builder::new()
      .name("executor".into())
      .spawn(move || {
        let mut executor = executor;
        executor.run().unwrap();
      })
      .into_diagnostic()
      .context("failed to launch executor thread")?;

    // run the winit event loop on the main thread for program lifecycle
    window_event_loop
      .run_app(&mut winit_app)
      .into_diagnostic()
      .context("failed to run winit event loop")?;

    // program exits
    Ok(())
  }

  /// Syntax sugar for sending a command
  fn command(&self, command: Command) {
    tracing::debug!(?command, "sending command");
    self.command_tx.send(command).unwrap();
  }

  /// Run the app event loop.
  pub fn run(&mut self) -> miette::Result<()> {
    while let Ok(event) = self.event_rx.recv() {
      let start = Instant::now();
      match event {
        // mainline event loop control flow
        Event::Windowing(box WindowingEvent::EventLoop(
          WinitEventLoopEvent::Resumed,
        )) => {
          debug!("received resumed event => building window");
          self
            .command(Command::EventLoopCommand(EventLoopCommand::BuildWindow));
        }
        Event::Windowing(box WindowingEvent::EventLoop(
          WinitEventLoopEvent::Suspended,
        )) => {
          debug!("received suspended event => destroying window");
          self.drop_window();
        }
        Event::Windowing(box WindowingEvent::EventLoop(
          WinitEventLoopEvent::Exiting,
        )) => {
          info!("winit event loop is exiting => ending app loop");
          break;
        }

        // resized
        Event::Windowing(box WindowingEvent::Window(
          _,
          WindowEvent::Resized(new_size),
        )) => {
          self.affect_resize(new_size);
        }
        // scale factor changed
        Event::Windowing(box WindowingEvent::Window(
          _,
          WindowEvent::ScaleFactorChanged { scale_factor, .. },
        )) => {
          self.affect_scale_factor_change(scale_factor);
        }
        // redraw requested
        Event::Windowing(box WindowingEvent::Window(
          _,
          WindowEvent::RedrawRequested,
        )) => {
          self.initiate_frame();
        }
        // close requested
        Event::Windowing(box WindowingEvent::Window(
          _,
          WindowEvent::CloseRequested,
        )) => {
          self.shut_down_app();
          return Ok(());
        }

        Event::Windowing(box WindowingEvent::Window(_, _window_event)) => {
          // tracing::debug!(window.id = ?w_id, "ignoring unimplemented window
          // event");
          if let Some(wh) = self.get_window_handle() {
            wh.request_redraw();
          }
        }
        Event::Windowing(box WindowingEvent::Device(_, _device_event)) => {
          // tracing::debug!(device.id = ?d_id, "ignoring unimplemented device
          // event");
          if let Some(wh) = self.get_window_handle() {
            wh.request_redraw();
          }
        }

        Event::Windowing(box WindowingEvent::WindowBuilt(window)) => {
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

      let elapsed = start.elapsed();
      if elapsed > Duration::from_micros(20) {
        warn!(
          "slow loop: event loop cycle took {:.03}ms (> 0.020 micros)",
          start.elapsed().as_millis_f32()
        );
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
    tracing::info!("shutting down app");
    self.drop_window();
    self.command(Command::EventLoopCommand(EventLoopCommand::ExitEventLoop));
  }

  fn initiate_frame(&self) {
    let frame_input = FrameInput {};

    let Some(window_handle) = self.get_window_handle() else {
      tracing::warn!("attempted to initiate a frame without a window present");
      return;
    };

    window_handle.initiate_frame(frame_input);
  }

  fn affect_resize(&self, new_size: PhysicalSize<u32>) {
    let Some(window_handle) = self.get_window_handle() else {
      tracing::warn!("attempted to affect a resize without a window present");
      return;
    };

    window_handle.handle_resize(new_size);
  }

  fn affect_scale_factor_change(&self, new_scale_factor: f64) {
    let Some(window_handle) = self.get_window_handle() else {
      tracing::warn!(
        "attempted to affect a scale factor change without a window present"
      );
      return;
    };

    window_handle.handle_scale_factor_change(new_scale_factor);
  }

  fn get_window_handle(&self) -> Option<&WindowHandle> {
    self.state.window.as_ref()
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
