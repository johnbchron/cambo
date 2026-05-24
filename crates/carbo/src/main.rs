#![feature(duration_millis_float)]

mod app;
mod draw;
mod event;
mod event_sender;
mod executor;
mod gpu_context;
mod renderer;
mod surface_state;
mod window_handle;
mod winit_app;

use miette::{Context, IntoDiagnostic};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
  EnvFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

fn main() -> miette::Result<()> {
  tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer())
    .with(
      EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy(),
    )
    .try_init()
    .into_diagnostic()?;

  let app_state =
    crate::app::AppState::build().context("failed to build app state")?;
  crate::app::App::launch(app_state)
}
