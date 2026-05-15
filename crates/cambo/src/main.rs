use miette::{Context, IntoDiagnostic};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
  EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};
use winit::event_loop::EventLoop;

use self::{gpu_context::GpuContext, window_app::App};

mod gpu_context;
mod window_app;
mod window_state;

fn main() -> miette::Result<()> {
  let gpu = GpuContext::new()?;
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(
      EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy(),
    )
    .try_init()
    .into_diagnostic()?;

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
