use wgpu::{Device, Extent3d, ImageCopyTexture, ImageDataLayout, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor};
use emulateme::rom::Rom;
use crate::hardware::palette::NES_PALETTE;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct HardwarePaletteMemory {
    pub indexes: [u32; 16]
}

pub struct SharedRenderer {
    pub pattern_texture_views: Vec<TextureView>,
    pub palette_texture_view: TextureView
}

impl SharedRenderer {
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

    // No draw.

    pub fn new(device: &Device, queue: &Queue, rom: &Rom) -> SharedRenderer {
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

        SharedRenderer {
            pattern_texture_views,
            palette_texture_view
        }
    }
}