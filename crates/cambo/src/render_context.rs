use std::sync::Arc;

use miette::{Context, IntoDiagnostic};

use crate::gpu_context::GpuContext;

pub struct RenderContext {
  renderer: vello::Renderer,
  gpu:      Arc<GpuContext>,
}

impl RenderContext {
  pub fn new(gpu: Arc<GpuContext>) -> miette::Result<Self> {
    let renderer = vello::Renderer::new(gpu.device(), vello::RendererOptions {
      use_cpu:              false,
      antialiasing_support: vello::AaSupport::area_only(),
      num_init_threads:     None,
      pipeline_cache:       None,
    })
    .into_diagnostic()
    .context("failed to create vello renderer")?;

    Ok(Self { renderer, gpu })
  }
}
