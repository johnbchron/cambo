use miette::{Context, IntoDiagnostic};
use wgpu::{
  Backends, Device, DeviceDescriptor, Instance, Queue, RequestAdapterOptions,
};

pub struct GpuContext {
  pub renderer: vello::Renderer,
  pub queue:    Queue,
  pub device:   Device,
  pub instance: Instance,
}

impl GpuContext {
  pub fn new() -> miette::Result<Self> {
    // no support for GL
    let instance_descriptor = wgpu::InstanceDescriptor {
      backends: Backends::PRIMARY,
      ..wgpu::InstanceDescriptor::from_env_or_default()
    };
    let instance = Instance::new(&instance_descriptor);

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
