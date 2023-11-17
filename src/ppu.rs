use std::error::Error;
use std::fmt::{Display, Formatter};
use crate::rom::Rom;

const SPRITE_COUNT: usize = 64;

#[derive(Clone, Debug)]
pub enum PpuMemoryError {
    UnmappedRead(u16),
    UnmappedWrite(u16),
    OamAccess(u8),
    PaletteAccess(u16),
}

pub struct NameTable {
    pub contents: [u8; 0x400]
}

pub type Palette = [u8; 3];

pub struct PaletteMemory {
    pub background_solid: u8,
    pub background: [Palette; 4],
    pub sprite: [Palette; 4],
}

#[derive(Default)]
pub struct ControlRegister {
    pub increment_32: bool,
    pub base_sprite_pattern_table: bool,
    pub base_background_pattern_table: bool,
    pub sprite_size: bool, // true = 8x16 pixels
    pub ppu_color_ext: bool,
    pub gen_nmi: bool,
}

#[derive(Default)]
pub struct MaskRegister {
    pub greyscale: bool,
    pub show_background_leftmost: bool,
    pub show_sprites_leftmost: bool,
    pub show_background: bool,
    pub show_sprites: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
    pub emphasize_blue: bool,
}

pub struct StatusRegister {
    pub sprite_hit: bool,
    pub v_blank_hit: bool,
}

#[derive(Default)]
pub struct RenderRegister {
    pub t: u16,
    pub v: u16,
    pub x: u8,
    pub w: bool
}

#[derive(Copy, Clone)]
pub struct Sprite {
    pub y: u8,
    pub number: u8,
    pub mask: u8,
    pub x: u8
}

#[derive(Default)]
pub struct PpuRegisters {
    pub control: ControlRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub render: RenderRegister,

    pub oam_address: u8,

    // pub write_low_address: bool,
    // pub address: u16,
    pub read_buffer: u8,
}

pub struct PpuMemory<'a> {
    pub rom: &'a Rom,

    pub oam: [Sprite; SPRITE_COUNT],
    pub names: [NameTable; 4],
    pub palette: PaletteMemory
}

pub struct Ppu<'a> {
    pub registers: PpuRegisters,
    pub memory: PpuMemory<'a>
}

impl Default for Sprite {
    fn default() -> Sprite {
        Sprite {
            y: 0xFF,
            number: 0,
            mask: 0,
            x: 0,
        }
    }
}

impl Sprite {
    pub fn read(&self, address: u8) -> u8 {
        match address {
            0 => self.y,
            1 => self.number,
            2 => self.mask,
            3 => self.x,
            _ => panic!("Unmapped read to sprite ${address:02X}")
        }
    }

    pub fn write(&mut self, address: u8, value: u8) {
        match address {
            0 => self.y = value,
            1 => self.number = value,
            2 => self.mask = value,
            3 => self.x = value,
            _ => panic!("Unmapped write to sprite ${address:02X}")
        }
    }
}

impl Default for StatusRegister {
    fn default() -> StatusRegister {
        StatusRegister {
            sprite_hit: false,
            v_blank_hit: true,
        }
    }
}

impl ControlRegister {
    pub fn from_bits(value: u8) -> ControlRegister {
        ControlRegister {
            increment_32: value & 0b00000100 != 0,
            base_sprite_pattern_table: value & 0b00001000 != 0,
            base_background_pattern_table: value & 0b00010000 != 0,
            sprite_size: value & 0b00100000 != 0,
            ppu_color_ext: value & 0b01000000 != 0,
            gen_nmi: value & 0b10000000 != 0,
        }
    }
}

impl MaskRegister {
    pub fn from_bits(value: u8) -> MaskRegister {
        MaskRegister {
            greyscale: value & 0b00000001 != 0,
            show_background_leftmost: value & 0b00000010 != 0,
            show_sprites_leftmost: value & 0b00000100 != 0,
            show_background: value & 0b00001000 != 0,
            show_sprites: value & 0b00010000 != 0,
            emphasize_red: value & 0b00100000 != 0,
            emphasize_green: value & 0b01000000 != 0,
            emphasize_blue: value & 0b10000000 != 0,
        }
    }
}

impl StatusRegister {
    pub fn bits(&self) -> u8 {
        let sprite_hit = if self.sprite_hit { 0b01000000 } else { 0 };
        let v_blank_hit = if self.v_blank_hit { 0b10000000 } else { 0 };

        sprite_hit | v_blank_hit
    }
}

impl RenderRegister {
    pub fn x_scroll(&self) -> u8 {
        (((self.t & 0b0000000000011111) as u8) << 3) | self.x
    }

    pub fn y_scroll(&self) -> u8 {
        (((self.t & 0b0000001111100000) >> 2) as u8) | (((self.t & 0b0111000000000000) >> 12) as u8)
    }

    pub fn name_table_x(&self) -> bool {
        self.t & 0b0000010000000000 != 0
    }

    pub fn name_table_y(&self) -> bool {
        self.t & 0b0000100000000000 != 0
    }

    pub fn write_control(&mut self, value: u8) {
        self.t = (self.t & 0b1111001111111111) | (((value & 0b11) as u16) << 10)
    }

    pub fn read_status(&mut self) {
        self.w = false;
    }

    pub fn write_scroll(&mut self, value: u8) {
        if self.w {
            let low = ((value & 0b111) as u16) << 12;
            let high = ((value & 0b11111000) as u16) << 2;

            self.t = (self.t & 0b0000110000011111) | low | high;
        } else {
            self.t = (self.t & 0b1111111111100000) | (((value as u16) & 0b11111000) >> 3);
            self.x = value & 0b111;
        }

        self.w = !self.w;
    }

    pub fn write_address(&mut self, value: u8) {
        if self.w {
            self.t = (self.t & 0b1111111100000000) | (value as u16);

            self.v = self.t
        } else {
            self.t = (self.t & 0b0000000011111111) | (((value & 0b00111111) as u16) << 8);
        }

        self.w = !self.w;
    }
}

impl Display for PpuMemoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PpuMemoryError::UnmappedRead(address) =>
                write!(f, "Unmapped PPU read at address ${address:04X}"),
            PpuMemoryError::UnmappedWrite(address) =>
                write!(f, "Unmapped PPU write at address ${address:04X}"),
            PpuMemoryError::PaletteAccess(address) =>
                write!(f, "Unexpected palette access at address ${address:04X}"),
            PpuMemoryError::OamAccess(address) =>
                write!(f, "Unexpected OAM access at address ${address:04X}"),
        }
    }
}

impl Error for PpuMemoryError { }

impl PaletteMemory {
    pub fn get(&self, address: u16) -> Result<u8, PpuMemoryError> {
        let address = address % 0x20;

        Ok(match address {
            0x00 => self.background_solid,
            0x01..=0x0F => {
                let base = (address - 0x3F01) as usize;
                let page = base / 4;
                let index = base % 4;

                self.background[page][index]
            }
            0x10 => self.background_solid,
            0x11..=0x1F => {
                let base = (address - 0x3F11) as usize;
                let page = base / 4;
                let index = base % 4;

                self.sprite[page][index]
            }
            _ => return Err(PpuMemoryError::PaletteAccess(address))
        })
    }

    pub fn set(&mut self, address: u16, value: u8) -> Result<(), PpuMemoryError> {
        let address = address % 0x20;

        match address {
            0x00 => self.background_solid = value,
            0x01..=0x0F => {
                let base = (address - 0x01) as usize;
                let page = base / 4;
                let index = base % 4;

                if index < 3 {
                    self.background[page][index] = value
                }
            }
            0x10 => self.background_solid = value,
            0x11..=0x1F => {
                let base = (address - 0x11) as usize;
                let page = base / 4;
                let index = base % 4;

                if index < 3 {
                    self.sprite[page][index] = value
                }
            }
            _ => return Err(PpuMemoryError::PaletteAccess(address))
        }

        Ok(())
    }
}

impl Default for PaletteMemory {
    fn default() -> PaletteMemory {
        PaletteMemory {
            background_solid: 0,
            background: std::array::from_fn(|_| Palette::default()),
            sprite: std::array::from_fn(|_| Palette::default()),
        }
    }
}

impl<'a> PpuMemory<'a> {
    pub fn read(&mut self, address: u16) -> Result<u8, PpuMemoryError> {
        Ok(match address {
            0x0000..=0x1FFF => self.rom.chr_rom[address as usize],
            0x2000..=0x3EFF => {
                let base = (address - 0x2000) as usize;
                let page = (base / 0x400) % 4;
                let index = base % 0x400;

                self.names[page].contents[index]
            }
            0x3F00..=0x3FFF => self.palette.get(address - 0x3F00)?,
            _ => return Err(PpuMemoryError::UnmappedRead(address))
        })
    }

    pub fn write(&mut self, address: u16, value: u8) -> Result<(), PpuMemoryError> {
        match address {
            0x2000..=0x3EFF => {
                let base = (address - 0x2000) as usize;
                let page = (base / 0x400) % 4;
                let index = base % 0x400;

                self.names[page].contents[index] = value
            }
            0x3F00..=0x3FFF => self.palette.set(address - 0x3F00, value)?,
            _ => return Err(PpuMemoryError::UnmappedWrite(address))
        }

        Ok(())
    }

    pub fn new(rom: &Rom) -> PpuMemory {
        PpuMemory {
            rom,

            oam: std::array::from_fn(|_| Sprite::default()),
            names: std::array::from_fn(|_| NameTable { contents: [0; 0x400] }),
            palette: PaletteMemory::default()
        }
    }
}

impl<'a> Ppu<'a> {
    pub fn write_ctrl(&mut self, value: u8) {
        self.registers.control = ControlRegister::from_bits(value);

        self.registers.render.write_control(value)
    }

    pub fn write_mask(&mut self, value: u8) {
        self.registers.mask = MaskRegister::from_bits(value);
    }

    pub fn read_status(&mut self) -> u8 {
        self.registers.render.read_status();

        self.registers.status.bits()
    }

    pub fn write_oam_address(&mut self, value: u8) {
        self.registers.oam_address = value;
    }

    pub fn read_oam_data(&mut self) -> u8 {
        let sprite = self.registers.oam_address / 4;
        let index = self.registers.oam_address % 4;

        self.memory.oam[sprite as usize].read(index)
    }

    pub fn write_oam_data(&mut self, value: u8) {
        let sprite = self.registers.oam_address / 4;
        let index = self.registers.oam_address % 4;

        self.memory.oam[sprite as usize].write(index, value);
    }

    pub fn write_scroll(&mut self, value: u8) {
        self.registers.render.write_scroll(value)
    }

    pub fn write_address(&mut self, value: u8) {
        self.registers.render.write_address(value)
    }

    pub fn write_data(&mut self, value: u8) -> Result<(), PpuMemoryError> {
        self.memory.write(self.registers.render.v, value)?;

        if self.registers.control.increment_32 {
            self.registers.render.v += 32;
        } else {
            self.registers.render.v += 1;
        }

        Ok(())
    }

    pub fn read_data(&mut self) -> Result<u8, PpuMemoryError> {
        let result = self.registers.read_buffer;

        self.registers.read_buffer = self.memory.read(self.registers.render.v)?;

        if self.registers.control.increment_32 {
            self.registers.render.v += 32;
        } else {
            self.registers.render.v += 1;
        }

        Ok(result)
    }

    pub fn replace_oam(&mut self, data: [u8; 256]) {
        self.memory.oam = std::array::from_fn(|i| {
            Sprite {
                y: data[i * 4],
                number: data[i * 4 + 1],
                mask: data[i * 4 + 2],
                x: data[i * 4 + 3],
            }
        });
    }

    pub fn new(rom: &Rom) -> Ppu {
        Ppu {
            registers: PpuRegisters::default(),
            memory: PpuMemory::new(rom)
        }
    }
}
