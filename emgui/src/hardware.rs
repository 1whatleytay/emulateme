use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use bitflags::Flags;
use wgpu::{Adapter, Buffer, BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Face, FragmentState, FrontFace, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, include_wgsl, Instance, InstanceDescriptor, LoadOp, Maintain, MaintainBase, MapMode, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use emulateme::ppu::Ppu;
use emulateme::renderer::{FrameRenderer, NES_HEIGHT, NES_WIDTH, RenderAction, RenderedFrame, Renderer};
use emulateme::software::{NES_SCANLINE_COUNT, NES_SCANLINE_WIDTH};
use anyhow::Result;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct BackgroundBasicVertex {
    position: [f32; 2]
}

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
    background_buffer: Buffer,
    background_pipeline: RenderPipeline,
    render_information: Arc<Mutex<RenderInformation>>
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
    pub fn render_contents(&mut self) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("HardwareRendererEncoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("HardwareRendererRenderPass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &self.render_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: StoreOp::Store,
                        },
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.background_pipeline);
            render_pass.set_vertex_buffer(0, self.background_buffer.slice(..));
            render_pass.draw(0 .. 6, 0 .. 1);
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

        if has_v_blank && ppu.registers.control.gen_nmi {
            self.render_contents();

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

impl<'a, 'b> HardwareRenderer<'a, 'b> {
    pub fn new(device: &'a Device, queue: &'b Queue) -> HardwareRenderer<'a, 'b> {
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

        let background_vertices = [
            BackgroundBasicVertex { position: [-1.0, 1.0] },
            BackgroundBasicVertex { position: [-1.0, -1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0] },
            BackgroundBasicVertex { position: [1.0, 1.0] },
            BackgroundBasicVertex { position: [-1.0, 1.0] },
        ];

        let background_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("BackgroundBufferRectangle"),
            contents: bytemuck::bytes_of(&background_vertices),
            usage: BufferUsages::VERTEX,
        });

        let render_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("RenderBufferDestination"),
            size: (NES_WIDTH * NES_HEIGHT * 4) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let background_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("BackgroundPipelineLayout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let background_shader = device.create_shader_module(include_wgsl!("background.wgsl"));

        let background_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("BackgroundPipeline"),
            layout: Some(&background_pipeline_layout),
            vertex: VertexState {
                module: &background_shader,
                entry_point: "vertex",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<BackgroundBasicVertex>() as u64,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[
                            VertexAttribute {
                                format: VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            }
                        ],
                    }
                ],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Front),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &background_shader,
                entry_point: "fragment",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        HardwareRenderer {
            last_cycle: 0,
            scan_x: 0,
            scan_y: 0,
            device, queue,
            background_buffer,
            render_texture,
            render_texture_view,
            background_pipeline,
            render_information: Arc::new(Mutex::new(RenderInformation {
                render_buffer,
                frame: None,
                waiting_frame: false
            })),
        }
    }
}
