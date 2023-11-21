use std::sync::{Arc, Mutex};
use anyhow::anyhow;
use bitflags::Flags;
use wgpu::{Adapter, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Face, FilterMode, FragmentState, FrontFace, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, include_wgsl, Instance, InstanceDescriptor, LoadOp, Maintain, MaintainBase, MapMode, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use emulateme::ppu::Ppu;
use emulateme::renderer::{FrameRenderer, NES_HEIGHT, NES_WIDTH, RenderAction, RenderedFrame, Renderer};
use emulateme::software::{NES_SCANLINE_COUNT, NES_SCANLINE_WIDTH};
use anyhow::Result;
use emulateme::rom::Rom;

const NES_PALETTE: [[u8; 4]; 0x40] = [
    [98, 98, 98, 255],
    [0, 31, 177, 255],
    [35, 3, 199, 255],
    [81, 0, 177, 255],
    [115, 0, 117, 255],
    [127, 0, 35, 255],
    [115, 10, 0, 255],
    [81, 39, 0, 255],
    [35, 67, 0, 255],
    [0, 86, 0, 255],
    [0, 92, 0, 255],
    [0, 82, 35, 255],
    [0, 60, 117, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
    [170, 170, 170, 255],
    [13, 86, 255, 255],
    [74, 47, 255, 255],
    [138, 18, 255, 255],
    [188, 8, 213, 255],
    [210, 17, 104, 255],
    [199, 45, 0, 255],
    [157, 84, 0, 255],
    [96, 123, 0, 255],
    [32, 151, 0, 255],
    [0, 162, 0, 255],
    [0, 152, 66, 255],
    [0, 124, 180, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
    [255, 255, 255, 255],
    [82, 174, 255, 255],
    [143, 133, 255, 255],
    [210, 101, 255, 255],
    [255, 86, 255, 255],
    [255, 93, 206, 255],
    [255, 119, 86, 255],
    [249, 158, 0, 255],
    [188, 199, 0, 255],
    [121, 231, 0, 255],
    [66, 246, 17, 255],
    [38, 239, 125, 255],
    [44, 213, 245, 255],
    [77, 77, 77, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
    [255, 255, 255, 255],
    [182, 225, 255, 255],
    [205, 208, 255, 255],
    [232, 195, 255, 255],
    [255, 187, 255, 255],
    [255, 188, 243, 255],
    [255, 198, 195, 255],
    [255, 213, 153, 255],
    [232, 230, 129, 255],
    [205, 243, 129, 255],
    [182, 250, 153, 255],
    [168, 249, 195, 255],
    [168, 240, 243, 255],
    [183, 183, 183, 255],
    [0, 0, 0, 255],
    [0, 0, 0, 255],
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
struct HardwarePaletteMemory {
    indexes: [u32; 16]
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct BackgroundBasicVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
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
    palette_memory_buffer: Buffer,
    name_table_textures: Vec<Texture>,
    binding_groups: Vec<BindGroup>,
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
    pub fn render_contents(&mut self, ppu: &Ppu) {
        self.device.poll(Maintain::Wait);

        for i in 0 .. 2 {
            self.queue.write_texture(ImageCopyTexture {
                texture: &self.name_table_textures[i],
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            }, &ppu.memory.names[i].contents, ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(32),
                rows_per_image: Some(30),
            }, Extent3d {
                width: 32,
                height: 30,
                depth_or_array_layers: 1,
            })
        }

        let hardware_palette = HardwarePaletteMemory {
            indexes: std::array::from_fn(|i| {
                let index = i % 4;

                if index == 0 {
                    return 0
                }

                ppu.memory.palette.background[i / 4][index - 1] as u32
            })
        };

        self.queue.write_buffer(&self.palette_memory_buffer, 0, bytemuck::bytes_of(&hardware_palette));

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

            render_pass.set_pipeline(&self.background_pipeline);
            render_pass.set_vertex_buffer(0, self.background_buffer.slice(..));
            render_pass.set_bind_group(0, &self.binding_groups[1], &[]);
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

impl<'a, 'b> HardwareRenderer<'a, 'b> {
    fn pattern_table_to_texture(data: &[u8]) -> Vec<u8> {
        // Hopefully this doesn't kill floating point accuracy.
        let sprite_width = 256 * 8;
        let sprite_height = 8;

        let mut result = vec![0; sprite_width * sprite_height];

        for i in 0 .. 0x100 {
            let sprite_start = i * 0x10;
            let sprite_horizontal_start = i * 0x8;

            let sprite = &data[sprite_start .. sprite_start + 0x10];

            for y in 0 .. 8 {
                let top = sprite[y];
                let bottom = sprite[y + 8];

                for x in 0 .. 8 {
                    let top_bit = (top >> (7 - x)) & 0b1;
                    let bottom_bit = (bottom >> (7 - x)) & 0b1;

                    let value = (bottom_bit << 1) | top_bit;

                    result[sprite_horizontal_start + x + y * sprite_width] = value;
                }
            }
        }

        result
    }

    pub fn new(device: &'a Device, queue: &'b Queue, rom: &'_ Rom) -> HardwareRenderer<'a, 'b> {
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
            BackgroundBasicVertex { position: [-1.0, 1.0], tex_coord: [0.0, 0.0] },
            BackgroundBasicVertex { position: [-1.0, -1.0], tex_coord: [0.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0], tex_coord: [1.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0], tex_coord: [1.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, 1.0], tex_coord: [1.0, 0.0] },
            BackgroundBasicVertex { position: [-1.0, 1.0], tex_coord: [0.0, 0.0] },
        ];

        let palette_texture = device.create_texture(&TextureDescriptor {
            label: Some("PaletteTexture"),
            size: Extent3d {
                width: 0x40,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D1,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let palette_texture_view = palette_texture.create_view(&TextureViewDescriptor::default());

        queue.write_texture(ImageCopyTexture {
            texture: &palette_texture,
            mip_level: 0,
            origin: Default::default(),
            aspect: Default::default(),
        }, bytemuck::bytes_of(&NES_PALETTE), ImageDataLayout {
            offset: 0,
            bytes_per_row: Some((std::mem::size_of::<[f32; 4]>() * 0x40) as u32),
            rows_per_image: Some(1),
        }, Extent3d {
            width: 0x40,
            height: 1,
            depth_or_array_layers: 1,
        });

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

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("BackgroundPipelineBindGroup"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let palette_memory_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("PaletteMemory"),
            size: std::mem::size_of::<HardwarePaletteMemory>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let background_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("BackgroundPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
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
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x2,
                                offset: std::mem::size_of::<[f32; 2]>() as u64,
                                shader_location: 1,
                            }
                        ],
                    }
                ],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
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

        let name_table_textures: Vec<Texture> = (0 .. 2).map(|i| {
            device.create_texture(&TextureDescriptor {
                label: Some(&format!("NT_{i}")),
                size: Extent3d {
                    width: 32,
                    height: 30,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R8Uint,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            })
        }).collect();

        let name_table_texture_views: Vec<TextureView> = name_table_textures.iter().enumerate().map(|(i, texture)| {
            texture.create_view(&TextureViewDescriptor::default())
        }).collect();

        let pattern_section_size = 0x1000;
        let pattern_textures: Vec<Texture> = (0 .. rom.chr_rom.len())
            .step_by(pattern_section_size)
            .map(|i| {
                let sprite_data = &rom.chr_rom[i .. i + pattern_section_size];
                let texture_data = Self::pattern_table_to_texture(sprite_data);

                let texture = device.create_texture(&TextureDescriptor {
                    label: Some(&format!("CHR_{i:04X}")),
                    size: Extent3d {
                        width: 256 * 8,
                        height: 8,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::R8Uint,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                    view_formats: &[],
                });

                queue.write_texture(ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Default::default(),
                    aspect: Default::default(),
                }, &texture_data, ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(256 * 8),
                    rows_per_image: Some(8),
                }, Extent3d {
                    width: 256 * 8,
                    height: 8,
                    depth_or_array_layers: 1,
                });

                texture
            }).collect();

        let pattern_texture_views: Vec<TextureView> = pattern_textures.iter().map(|texture| {
            texture.create_view(&TextureViewDescriptor::default())
        }).collect();

        let binding_groups: Vec<BindGroup> = pattern_texture_views.iter().enumerate().map(|(i, texture_view)| {
            device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("CHR_i{i}_bind_group")),
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&name_table_texture_views[0]),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&name_table_texture_views[1]),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&palette_texture_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: palette_memory_buffer.as_entire_binding()
                    }
                ],
            })
        }).collect();

        HardwareRenderer {
            last_cycle: 0,
            scan_x: 0,
            scan_y: 0,
            device, queue,
            background_buffer,
            name_table_textures,
            palette_memory_buffer,
            binding_groups,
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
