use std::cmp::min;
use wgpu::*;
use anyhow::Result;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct StreamerVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

pub struct StreamerDetails {
    pub surface: Surface,
    pub format: TextureFormat,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue
}

pub struct Streamer<'a> {
    width: usize,
    height: usize,
    details: &'a StreamerDetails,
    buffer: Buffer,
    texture: Texture,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

struct BoxedSize {
    x: u64,
    y: u64,
    width: u64,
    height: u64
}

impl BoxedSize {
    pub fn new(width: u64, height: u64) -> BoxedSize {
        let minimum = min(width, height);

        BoxedSize {
            x: (width - minimum) / 2,
            y: (height - minimum) / 2,
            width: minimum,
            height: minimum,
        }
    }
}

impl<'a> Streamer<'a> {
    pub fn render_frame(&self, data: &[u8], window_size: PhysicalSize<u32>) -> Result<()> {
        assert_eq!(data.len(), self.width * self.height * 4);

        self.details.queue.write_texture(ImageCopyTexture {
            texture: &self.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        }, data, ImageDataLayout {
            offset: 0,
            bytes_per_row: Some((self.width * 4) as u32),
            rows_per_image: Some(self.height as u32),
        }, Extent3d {
            width: self.width as u32,
            height: self.height as u32,
            depth_or_array_layers: 1
        });

        self.redraw_frame(window_size)
    }

    pub fn redraw_frame(&self, window_size: PhysicalSize<u32>) -> Result<()> {
        let frame = self.details.surface.get_current_texture()?;
        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut commands = self.details.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("RenderFrameCommandEncoder"),
        });

        {
            let mut render_pass = commands.begin_render_pass(&RenderPassDescriptor {
                label: Some("RenderFrameRenderPass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &frame_view,
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

            // 1600 1200
            let boxed_size = BoxedSize::new(window_size.width as u64, window_size.height as u64);

            render_pass.set_viewport(
                boxed_size.x as f32, boxed_size.y as f32,
                boxed_size.width as f32, boxed_size.height as f32,
                0.0, 1.0
            );
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        self.details.queue.submit(std::iter::once(commands.finish()));
        frame.present();

        Ok(())
    }

    pub fn new(details: &'a StreamerDetails, width: usize, height: usize) -> Streamer {
        let data = vec![
            StreamerVertex { position: [-1.0, -1.0], tex_coords: [0.0, 1.0] },
            StreamerVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
            StreamerVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
            StreamerVertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] },
            StreamerVertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] },
            StreamerVertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] },
        ];

        let buffer = details.device.create_buffer(&BufferDescriptor {
            label: Some("MainBuffer"),
            size: (data.len() * std::mem::size_of::<StreamerVertex>()) as u64,
            usage: BufferUsages::VERTEX,
            mapped_at_creation: true,
        });

        buffer.slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(&data));
        buffer.unmap();

        let texture = details.device.create_texture(&TextureDescriptor {
            label: Some("StreamingTexture"),
            size: Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = details.device.create_sampler(&SamplerDescriptor {
            label: Some("MainSampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let bind_group_layout = details.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("MainBindGroup"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                }
            ],
        });

        let bind_group = details.device.create_bind_group(&BindGroupDescriptor {
            label: Some("MainBindGroup"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                }
            ],
        });

        let shader = details.device.create_shader_module(include_wgsl!("shader.wgsl"));

        let pipeline_layout = details.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("MainPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_state = VertexState {
            module: &shader,
            entry_point: "vertex",
            buffers: &[
                VertexBufferLayout {
                    array_stride: std::mem::size_of::<StreamerVertex>() as u64,
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
        };

        let fragment_state = FragmentState {
            module: &shader,
            entry_point: "fragment",
            targets: &[Some(ColorTargetState {
                format: details.format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        };

        let primitive_state = PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            // cull_mode: Some(Face::Back),
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        };

        let pipeline = details.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("MainRenderPipeline"),
            layout: Some(&pipeline_layout),
            vertex: vertex_state,
            primitive: primitive_state,
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(fragment_state),
            multiview: None,
        });

        Streamer {
            width,
            height,
            details,
            buffer,
            texture,
            bind_group,
            pipeline,
        }
    }
}
