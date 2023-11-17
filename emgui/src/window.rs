use std::cell::Cell;
use std::sync::Arc;
use winit::window::{Window, WindowBuilder};
use anyhow::{anyhow, Result};
use bitflags::Flags;
use wgpu::{CompositeAlphaMode, DeviceDescriptor, Instance, InstanceDescriptor, PresentMode, RequestAdapterOptions, SurfaceConfiguration, TextureFormat, TextureUsages};
use winit::dpi::PhysicalSize;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use crate::streamer::StreamerDetails;

pub struct WindowDetails {
    pub window: Arc<Window>,
    pub instance: Instance,
    pub size: Cell<PhysicalSize<u32>>,
    pub details: StreamerDetails
}

pub async fn create_streamer_details(instance: &Instance, window: &Window) -> Result<StreamerDetails> {
    let surface = unsafe { instance.create_surface(&window) }?;

    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }).await.ok_or_else(|| anyhow!("Failed to create adapter."))?;

    let (device, queue) = adapter.request_device(&DeviceDescriptor {
        label: Some("PrimaryDevice"),
        features: Default::default(),
        limits: Default::default(),
    }, None).await?;

    let capabilities = surface.get_capabilities(&adapter);

    let format = capabilities.formats.iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(TextureFormat::Rgba8Unorm);

    Ok(StreamerDetails {
        surface,
        adapter,
        device,
        queue,
        format
    })
}

pub fn configure_surface(details: &StreamerDetails, size: PhysicalSize<u32>) {
    details.surface.configure(&details.device, &SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: details.format,
        width: size.width,
        height: size.height,
        present_mode: PresentMode::AutoVsync,
        alpha_mode: CompositeAlphaMode::Auto,
        view_formats: vec![],
    })
}

impl WindowDetails {
    pub fn run<F: FnMut(), G: FnMut(KeyEvent)>(&self, event_loop: EventLoop<()>, mut render: F, mut key: G) -> Result<()> {
        event_loop.run(|event, target| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(size) => {
                        self.size.set(size);

                        configure_surface(&self.details, size);
                        self.window.request_redraw();
                    }

                    WindowEvent::ScaleFactorChanged { .. } => {
                        let size = self.window.inner_size();
                        self.size.set(size);

                        configure_surface(&self.details, size);
                        self.window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        render()
                    }

                    WindowEvent::KeyboardInput { event, .. } => {
                        key(event)
                    }

                    _ => { }
                }
            }
        })?;

        Ok(())
    }

    pub fn make(title: &str) -> Result<(WindowDetails, EventLoop<()>)> {
        env_logger::init();

        let event_loop = EventLoop::new()?;

        let window = WindowBuilder::new()
            .with_title(title)
            .build(&event_loop)?;

        let instance = Instance::new(InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: Default::default(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });

        let details = pollster::block_on(create_streamer_details(&instance, &window))?;
        let size = window.inner_size();

        configure_surface(&details, size);

        let details = WindowDetails {
            window: Arc::new(window),
            instance,
            size: Cell::new(size),
            details
        };

        Ok((details, event_loop))
    }
}
