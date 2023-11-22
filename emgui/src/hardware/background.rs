use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d, Face, FragmentState, FrontFace, ImageCopyTexture, ImageDataLayout, include_wgsl, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use emulateme::ppu::Ppu;
use crate::hardware::shared::{HardwarePaletteMemory, SharedRenderer};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct BackgroundBasicVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

pub struct BackgroundRenderer {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    bind_groups: Vec<BindGroup>,

    name_table_textures: Vec<Texture>,
    palette_memory_buffer: Buffer,
}

impl BackgroundRenderer {
    pub fn prepare(&self, ppu: &Ppu, queue: &Queue) {
        let hardware_palette = HardwarePaletteMemory {
            indexes: std::array::from_fn(|i| {
                let index = i % 4;

                if index == 0 {
                    return 0
                }

                ppu.memory.palette.background[i / 4][index - 1] as u32
            })
        };

        queue.write_buffer(&self.palette_memory_buffer, 0, bytemuck::bytes_of(&hardware_palette));

        for i in 0 .. 2 {
            queue.write_texture(ImageCopyTexture {
                texture: &self.name_table_textures[i],
                mip_level: 0,
                origin: Default::default(),
                aspect: Default::default(),
            }, &ppu.memory.names[i].contents, ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(0x400),
                rows_per_image: Some(1),
            }, Extent3d {
                width: 0x400,
                height: 1,
                depth_or_array_layers: 1,
            })
        }

    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_groups[1], &[]);
        render_pass.draw(0 .. 6, 0 .. 1);
    }

    pub fn new(device: &Device, _: &Queue, shared: &SharedRenderer) -> BackgroundRenderer {
        let background_vertices = [
            BackgroundBasicVertex { position: [-1.0, 1.0], tex_coord: [0.0, 0.0] },
            BackgroundBasicVertex { position: [-1.0, -1.0], tex_coord: [0.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0], tex_coord: [1.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, -1.0], tex_coord: [1.0, 1.0] },
            BackgroundBasicVertex { position: [1.0, 1.0], tex_coord: [1.0, 0.0] },
            BackgroundBasicVertex { position: [-1.0, 1.0], tex_coord: [0.0, 0.0] },
        ];

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("BackgroundBufferRectangle"),
            contents: bytemuck::bytes_of(&background_vertices),
            usage: BufferUsages::VERTEX,
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
                        view_dimension: TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D1,
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
            label: Some("BackgroundPaletteMemory"),
            size: std::mem::size_of::<HardwarePaletteMemory>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let background_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("BackgroundPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(include_wgsl!("background.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("BackgroundPipeline"),
            layout: Some(&background_pipeline_layout),
            vertex: VertexState {
                module: &shader,
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
                module: &shader,
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
                    width: 0x400,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D1,
                format: TextureFormat::R8Uint,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            })
        }).collect();

        let name_table_texture_views: Vec<TextureView> = name_table_textures.iter().map(|texture| {
            texture.create_view(&TextureViewDescriptor::default())
        }).collect();

        let bind_groups: Vec<BindGroup> = shared.pattern_texture_views.iter().enumerate().map(|(i, texture_view)| {
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
                        resource: BindingResource::TextureView(&shared.palette_texture_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: palette_memory_buffer.as_entire_binding()
                    }
                ],
            })
        }).collect();

        BackgroundRenderer {
            pipeline,
            vertex_buffer,
            bind_groups,

            name_table_textures,
            palette_memory_buffer,
        }
    }
}
