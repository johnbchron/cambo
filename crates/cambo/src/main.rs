use miette::{Context, IntoDiagnostic};
use winit::event_loop::EventLoop;

use self::{gpu_context::GpuContext, window_app::App};

mod gpu_context;
mod window_app;
mod window_state;

fn main() -> miette::Result<()> {
  let gpu = GpuContext::new()?;
  let event_loop = EventLoop::new()
    .into_diagnostic()
    .context("failed to build winit event loop")?;
  let mut app = App { gpu, window: None };
  event_loop
    .run_app(&mut app)
    .into_diagnostic()
    .context("failed to run winit event loop")?;
  Ok(())
}
