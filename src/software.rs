use crate::ppu::{Palette, Ppu};
use crate::renderer::{Renderer, RenderAction};

pub const NES_WIDTH: usize = 256;
pub const NES_HEIGHT: usize = 240;

pub const NES_FRAME_SIZE: usize = NES_WIDTH * NES_HEIGHT * 4;

pub const NES_SCANLINE_WIDTH: usize = 341;
pub const NES_SCANLINE_COUNT: usize = 262;

pub struct RenderedFrame {
    pub frame: [u8; NES_FRAME_SIZE]
}

type Color = [u8; 4];

const NES_PALETTE: [Color; 0x40] = [
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

struct PreRenderedScanline {
    background: [Option<Color>; NES_WIDTH],
    foreground: [Option<Color>; NES_WIDTH]
}

pub struct SoftwareRenderer<F: FnMut(RenderedFrame)> {
    pub scan_x: usize,
    pub scan_y: usize,
    last_cycle: u64,
    pre_rendered_sprites: Option<PreRenderedScanline>,
    frame: RenderedFrame,
    push_frame: F
}

impl Default for RenderedFrame {
    fn default() -> RenderedFrame {
        RenderedFrame { frame: [255; NES_FRAME_SIZE] }
    }
}

impl Default for PreRenderedScanline {
    fn default() -> PreRenderedScanline {
        PreRenderedScanline {
            background: [None; NES_WIDTH],
            foreground: [None; NES_WIDTH],
        }
    }
}

impl<F: Fn(RenderedFrame)> SoftwareRenderer<F> {
    fn render_sprite(&mut self, ppu: &mut Ppu, sprite: usize, x: usize, y: usize, palette: Palette) -> Option<Color> {
        let address = sprite * 8 * 2 + y;
        let plane_0 = ppu.memory.rom.chr_rom[address];
        let plane_1 = ppu.memory.rom.chr_rom[address + 8];

        let mask = 1 << (7 - x);

        let has_bit_0 = plane_0 & mask != 0;
        let has_bit_1 = plane_1 & mask != 0;

        let index = if has_bit_0 { 1 } else { 0 } | if has_bit_1 { 2 } else { 0 };

        if index == 0 {
            None
        } else {
            let color_index = palette[index - 1];

            Some(NES_PALETTE[color_index as usize])
        }
    }

    fn render_background(&mut self, ppu: &mut Ppu, table: usize, x: usize, y: usize) -> Option<Color> {
        let col = x / 8;
        let row = y / 8;

        let col_sub = x % 8;
        let row_sub = y % 8;

        let sprite = ppu.memory.names[table].contents[col + row * 32];

        let attribute_column = col / 4;
        let attribute_row = row / 4;

        let attribute_address = 0x3C0 + attribute_column + attribute_row * 8;

        let attribute_byte = ppu.memory.names[table].contents[attribute_address];
        let attribute_right = (col / 2) % 2;
        let attribute_bottom = (row / 2) % 2;

        let attribute_shift = attribute_right * 2 + attribute_bottom * 4;
        let palette_index = (attribute_byte >> attribute_shift) & 0b11;

        let palette = ppu.memory.palette.background[palette_index as usize];

        self.render_sprite(ppu, sprite as usize + 256, col_sub, row_sub, palette)
    }

    fn pre_render_sprites(&mut self, ppu: &mut Ppu, y: usize) -> PreRenderedScanline {
        let mut result = PreRenderedScanline::default();

        let sprite_width = 8;
        let sprite_height = 8;

        for i in (0 .. 64).rev() {
            let sprite = ppu.memory.oam[i];

            // Sprites are delayed by one scanline.
            let sprite_y = sprite.y as usize + 1;

            if !(sprite_y <= y && y < sprite_y + sprite_height) {
                continue
            }

            let behind_background = sprite.mask & 0b00100000 != 0;

            let flip_x = sprite.mask & 0b01000000 != 0;
            let flip_y = sprite.mask & 0b10000000 != 0;

            let offset_y = y - sprite_y;

            let palette_index = sprite.mask & 0b11;
            let palette = ppu.memory.palette.sprite[palette_index as usize];

            for offset_x in 0 .. sprite_width {
                let write_x = sprite.x as usize + offset_x;

                if write_x >= NES_WIDTH {
                    break
                }

                let sprite_offset_x = if flip_x { sprite_width - 1 - offset_x } else { offset_x };
                let sprite_offset_y = if flip_y { sprite_height - 1 - offset_y } else { offset_y };

                let color = self.render_sprite(
                    ppu, sprite.number as usize, sprite_offset_x, sprite_offset_y, palette
                );

                if let Some(color) = color {
                    if i == 0 {
                        ppu.registers.status.sprite_hit = true;
                    }

                    if behind_background {
                        result.background[write_x] = Some(color);
                    } else {
                        result.foreground[write_x] = Some(color);
                    }
                }
            }
        }

        result
    }

    fn render_pixel(&mut self, ppu: &mut Ppu, x: usize, y: usize) -> Color {
        let foreground_pixel = self.pre_rendered_sprites.as_ref()
            .and_then(|pixels| pixels.foreground[x]);

        if let Some(color) = foreground_pixel {
            return color
        }

        let mut offset_x = x + (ppu.registers.scroll.x as usize);
        let mut offset_y = y + (ppu.registers.scroll.y as usize);

        let mut name_table = ppu.registers.control.base_name_table_x != ppu.registers.control.base_name_table_y;

        if offset_x >= 256 {
            offset_x -= 256;

            name_table = !name_table;
        }

        if offset_y >= 240 {
            offset_y -= 256;

            name_table = !name_table;
        }

        let name_table = if name_table { 1 } else { 0 };

        self.render_background(ppu, name_table, offset_x, offset_y)
            .or_else(|| {
                self.pre_rendered_sprites.as_ref()
                    .and_then(|pixels| pixels.background[x])
            })
            .unwrap_or_else(|| NES_PALETTE[ppu.memory.palette.background_solid as usize])
    }

    pub fn new(push_frame: F) -> SoftwareRenderer<F> {
        SoftwareRenderer {
            scan_x: 0, scan_y: 0,
            last_cycle: 0,
            pre_rendered_sprites: None,
            frame: RenderedFrame::default(),
            push_frame
        }
    }
}

impl<F: Fn(RenderedFrame)> Renderer for SoftwareRenderer<F> {
    fn render(&mut self, ppu: &mut Ppu, cycle: u64) -> RenderAction {
        let diff = (cycle - self.last_cycle) * 3;
        self.last_cycle = cycle;

        let mut has_v_blank = false;

        for _ in 0..diff {
            match self.scan_y {
                0 ..= 239 => {
                    if self.scan_x == 0 {
                        self.pre_rendered_sprites = Some(self.pre_render_sprites(ppu, self.scan_y));
                    }

                    if (1 ..= 256).contains(&self.scan_x) {
                        let x = self.scan_x - 1;

                        let pixel = self.render_pixel(ppu, x, self.scan_y);

                        let address = (x + self.scan_y * NES_WIDTH) * 4;

                        self.frame.frame[address .. address + 4].copy_from_slice(&pixel);
                    }
                }
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
            (self.push_frame)(std::mem::take(&mut self.frame));

            RenderAction::SendNMI
        } else {
            RenderAction::None
        }
    }
}
