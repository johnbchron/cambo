use miette::{Context, IntoDiagnostic};
use wgpu::{Device, DeviceDescriptor, Instance, Queue, RequestAdapterOptions};

pub struct GpuContext {
  pub instance: Instance,
  pub device:   Device,
  pub queue:    Queue,
  pub renderer: vello::Renderer,
}

impl GpuContext {
  pub fn new() -> miette::Result<Self> {
    let instance =
      Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());

    let (device, queue) = pollster::block_on(async {
      let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .expect("no suitable adapter");
      let (device, queue) = adapter
        .request_device(&DeviceDescriptor::default())
        .await
        .expect("failed to create device");
      (device, queue)
    });

    let renderer = vello::Renderer::new(&device, vello::RendererOptions {
      use_cpu:              false,
      antialiasing_support: vello::AaSupport::area_only(),
      num_init_threads:     None,
      pipeline_cache:       None,
    })
    .into_diagnostic()
    .context("failed to create vello renderer")?;

    Ok(Self {
      instance,
      device,
      queue,
      renderer,
    })
  }
}
