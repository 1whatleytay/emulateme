use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, CompareFunction, DepthStencilState, Device, FragmentState, FrontFace, include_wgsl, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use emulateme::ppu::Ppu;
use crate::hardware::shared::{HardwarePaletteMemory, SharedRenderer};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct SpriteData {
    y: u8,
    number: u8,
    mask: u8,
    x: u8
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct SpriteBasicVertex {
    position: [f32; 2],
    tex_coord: [f32; 2]
}

pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    oam_buffer: Buffer,
    bind_groups: Vec<BindGroup>,
    palette_memory_buffer: Buffer,
}

impl SpriteRenderer {
    pub fn prepare(&self, ppu: &Ppu, queue: &Queue) {
        let hardware_palette = HardwarePaletteMemory {
            indexes: std::array::from_fn(|i| {
                let index = i % 4;

                if index == 0 {
                    return 0
                }

                ppu.memory.palette.sprite[i / 4][index - 1] as u32
            })
        };

        queue.write_buffer(&self.palette_memory_buffer, 0, bytemuck::bytes_of(&hardware_palette));

        let mut sprite_data: [[u8; 4]; 64] = [[0, 0, 0, 0]; 64];

        // Hopefully this doesn't take too long.
        for (i, sprite) in ppu.memory.oam.iter().enumerate() {
            sprite_data[i] = [
                sprite.y,
                sprite.number,
                sprite.mask,
                sprite.x,
            ];
        }

        queue.write_buffer(&self.oam_buffer, 0, bytemuck::bytes_of(&sprite_data));
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_groups[0], &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.oam_buffer.slice(..));
        render_pass.draw(0 .. 6, 0 .. 64);
    }

    pub fn new(device: &Device, queue: &Queue, shared: &SharedRenderer) -> SpriteRenderer {
        let sw = 8f32 / 256f32 * 2f32;
        let sh = 8f32 / 240f32 * 2f32;

        let vertices = [
            SpriteBasicVertex { position: [0.0, 0.0], tex_coord: [0.0, 0.0] },
            SpriteBasicVertex { position: [0.0, -sh], tex_coord: [0.0, 1.0] },
            SpriteBasicVertex { position: [sw, 0.0], tex_coord: [1.0, 0.0] },
            SpriteBasicVertex { position: [sw, -sh], tex_coord: [1.0, 1.0] },
            SpriteBasicVertex { position: [sw, 0.0], tex_coord: [1.0, 0.0] },
            SpriteBasicVertex { position: [0.0, -sh], tex_coord: [0.0, 1.0] },
        ];

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("SpriteVertexBuffer"),
            contents: bytemuck::bytes_of(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(include_wgsl!("sprite.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SpriteBindGroupLayout"),
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
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
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

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("SpritePipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("SpriteRenderPass"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteBasicVertex>() as u64,
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
                    },
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteData>() as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: &[
                            VertexAttribute {
                                format: VertexFormat::Uint8x4,
                                offset: 0,
                                shader_location: 2,
                            }
                        ],
                    }
                ],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None, // CHANGE LATER
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fragment",
                targets: &[
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: Default::default(),
                    })
                ],
            }),
            multiview: None,
        });

        let palette_memory_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("SpritePaletteMemory"),
            size: std::mem::size_of::<HardwarePaletteMemory>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let bind_groups: Vec<BindGroup> = shared.pattern_texture_views.iter().enumerate().map(|(i, view)| {
            device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("SpriteBindGroup_{i}")),
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&shared.palette_texture_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Buffer(palette_memory_buffer.as_entire_buffer_binding()),
                    }
                ],
            })
        }).collect();

        let oam_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("OAMBuffer"),
            size: 256,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        SpriteRenderer {
            pipeline,
            vertex_buffer,
            oam_buffer,
            bind_groups,
            palette_memory_buffer,
        }
    }
}
