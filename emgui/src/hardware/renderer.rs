use std::sync::{Arc, Mutex};
use anyhow::{anyhow, Result};
use bitflags::Flags;
use wgpu::{Adapter, Buffer, BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Instance, InstanceDescriptor, LoadOp, Maintain, MapMode, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor};
use emulateme::ppu::Ppu;
use emulateme::renderer::{FrameRenderer, NES_HEIGHT, NES_WIDTH, RenderAction, RenderedFrame, Renderer};
use emulateme::rom::Rom;
use emulateme::software::{NES_SCANLINE_COUNT, NES_SCANLINE_WIDTH};
use crate::hardware::background::BackgroundRenderer;
use crate::hardware::palette::NES_PALETTE;
use crate::hardware::shared::SharedRenderer;
use crate::hardware::sprite::SpriteRenderer;

struct RenderInformation {
    render_buffer: Buffer,
    frame: Option<Box<RenderedFrame>>,
    waiting_frame: bool,
}

pub struct HardwareRenderer<'a, 'b> {
    last_cycle: u64,
    scan_x: usize,
    scan_y: usize,
    device: &'a Device,
    queue: &'b Queue,
    render_texture: Texture,
    render_texture_view: TextureView,
    render_information: Arc<Mutex<RenderInformation>>,

    shared: SharedRenderer,
    background: BackgroundRenderer,
    sprite: SpriteRenderer,

    // palette_memory_buffer: Buffer,
    // name_table_textures: Vec<Texture>,
    // binding_groups: Vec<BindGroup>,
    // render_texture: Texture,
    // render_texture_view: TextureView,
    // background_buffer: Buffer,
    // background_pipeline: RenderPipeline,
}

pub struct DeviceDetails {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

pub async fn create_device() -> Result<DeviceDetails> {
    let instance = Instance::new(InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: Default::default(),
        dx12_shader_compiler: Default::default(),
        gles_minor_version: Default::default(),
    });

    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        force_fallback_adapter: false,
        compatible_surface: None,
    }).await.ok_or_else(|| anyhow!("Failed to create adapter."))?;

    let (device, queue) = adapter.request_device(&DeviceDescriptor {
        label: Some("PrimaryDevice"),
        features: Default::default(),
        limits: Default::default(),
    }, None).await?;

    Ok(DeviceDetails {
        instance,
        adapter,
        device,
        queue,
    })
}

impl<'a, 'b> HardwareRenderer<'a, 'b> {
    pub fn render_contents(&mut self, ppu: &Ppu) {
        self.device.poll(Maintain::Wait);

        self.background.prepare(ppu, self.queue);
        self.sprite.prepare(ppu, self.queue);

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("HardwareRendererEncoder"),
        });

        {
            let background_color = ppu.memory.palette.background_solid;

            let color = NES_PALETTE[background_color as usize];

            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("HardwareRendererRenderPass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.render_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: color[0] as f64 / 255.0,
                                g: color[1] as f64 / 255.0,
                                b: color[2] as f64 / 255.0,
                                a: color[3] as f64 / 255.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.background.draw(&mut render_pass);
            self.sprite.draw(&mut render_pass);
        }

        {
            encoder.copy_texture_to_buffer(ImageCopyTexture {
                texture: &self.render_texture,
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            }, ImageCopyBuffer {
                buffer: &self.render_information.lock().unwrap().render_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((NES_WIDTH * 4) as u32),
                    rows_per_image: Some(NES_HEIGHT as u32),
                },
            }, Extent3d {
                width: NES_WIDTH as u32,
                height: NES_HEIGHT as u32,
                depth_or_array_layers: 1,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        let mut render_information = self.render_information.lock().unwrap();
        if !render_information.waiting_frame {
            render_information.waiting_frame = true;
            let render_clone = Arc::downgrade(&self.render_information);

            render_information.render_buffer.slice(..).map_async(MapMode::Read, move |result| {
                result.unwrap();

                let Some(render_information) = render_clone.upgrade() else { return };
                let mut render_information = render_information.lock().unwrap();

                let mut frame = Box::<RenderedFrame>::default();

                {
                    let mapping = render_information.render_buffer.slice(..)
                        .get_mapped_range();

                    frame.frame.copy_from_slice(&mapping[..]);
                }

                render_information.render_buffer.unmap();
                render_information.waiting_frame = false;

                render_information.frame = Some(frame);
            });
        }
    }

    pub fn new(device: &'a Device, queue: &'b Queue, rom: &Rom) -> HardwareRenderer<'a, 'b> {
        let render_texture = device.create_texture(&TextureDescriptor {
            label: Some("RenderingTexture"),
            size: Extent3d {
                width: 256,
                height: 240,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let render_texture_view = render_texture.create_view(&TextureViewDescriptor::default());

        let render_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("RenderBufferDestination"),
            size: (NES_WIDTH * NES_HEIGHT * 4) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let render_information = Arc::new(Mutex::new(RenderInformation {
            render_buffer,
            frame: None,
            waiting_frame: false
        }));

        let shared = SharedRenderer::new(device, queue, rom);
        let background = BackgroundRenderer::new(device, queue, &shared);
        let sprite = SpriteRenderer::new(device, queue, &shared);

        HardwareRenderer {
            last_cycle: 0,
            scan_x: 0,
            scan_y: 0,
            device,
            queue,
            render_texture,
            render_information,
            render_texture_view,
            shared,
            background,
            sprite,
        }
    }
}


impl<'a, 'b> Renderer for HardwareRenderer<'a, 'b> {
    fn sync(&mut self, cycles: u64) {
        self.last_cycle = cycles;
    }

    fn render(&mut self, ppu: &mut Ppu, cycle: u64) -> RenderAction {
        let diff = (cycle - self.last_cycle) * 3;
        self.last_cycle = cycle;

        let mut has_v_blank = false;

        for _ in 0..diff {
            match self.scan_y {
                0 ..= 239 => { }
                241 => {
                    if self.scan_x == 1 {
                        has_v_blank = true;
                    }
                }
                261 => {
                    if self.scan_x == 1 {
                        ppu.registers.status.sprite_hit = false;
                    }
                }
                _ => { /* idle */ }
            }

            self.scan_x += 1;

            if self.scan_x >= NES_SCANLINE_WIDTH {
                self.scan_x = 0;
                self.scan_y += 1;

                if self.scan_y >= NES_SCANLINE_COUNT {
                    self.scan_y = 0;
                }
            }
        }

        ppu.registers.status.sprite_hit = !ppu.registers.status.sprite_hit;

        if has_v_blank && ppu.registers.control.gen_nmi {
            self.render_contents(ppu);

            RenderAction::SendNMI
        } else {
            RenderAction::None
        }
    }
}

impl<'a, 'b> FrameRenderer for HardwareRenderer<'a, 'b> {
    fn take(&mut self) -> Option<Box<RenderedFrame>> {
        self.render_information.lock().unwrap().frame.take()
    }
}