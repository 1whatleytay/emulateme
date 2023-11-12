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
    pub base_name_table_x: bool,
    pub base_name_table_y: bool,
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

#[derive(Default)]
pub struct ScrollRegister {
    pub write_y: bool,

    pub x: u8,
    pub y: u8,
}

pub struct StatusRegister {
    pub sprite_hit: bool,
    pub v_blank_hit: bool,
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
    pub scroll: ScrollRegister,
    pub status: StatusRegister,

    pub oam_address: u8,

    pub write_low_address: bool,
    pub address: u16,
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
            base_name_table_x: value & 0b00000001 != 0,
            base_name_table_y: value & 0b00000010 != 0,
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

        println!("Write control {value:02X} -> base_nt = {}", self.registers.control.base_name_table_x);
    }

    pub fn write_mask(&mut self, value: u8) {
        self.registers.mask = MaskRegister::from_bits(value);
    }

    pub fn read_status(&mut self) -> u8 {
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
        if self.registers.scroll.write_y {
            self.registers.scroll.y = value;
        } else {
            self.registers.scroll.x = value;
        }

        println!("Write scroll {value} -> {}, {}", self.registers.scroll.x, self.registers.scroll.y);

        self.registers.scroll.write_y = !self.registers.scroll.write_y;
    }

    pub fn write_address(&mut self, value: u8) {
        if self.registers.write_low_address {
            self.registers.address = (self.registers.address & 0xFF00) | (value as u16);
        } else {
            self.registers.address = (self.registers.address & 0x00FF) | ((value as u16) << 8);
        }

        self.registers.write_low_address = !self.registers.write_low_address;
    }

    pub fn write_data(&mut self, value: u8) -> Result<(), PpuMemoryError> {
        self.memory.write(self.registers.address, value)?;

        if self.registers.control.increment_32 {
            self.registers.address += 32;
        } else {
            self.registers.address += 1;
        }

        Ok(())
    }

    pub fn read_data(&mut self) -> Result<u8, PpuMemoryError> {
        let result = self.registers.read_buffer;

        self.registers.read_buffer = self.memory.read(self.registers.address)?;

        if self.registers.control.increment_32 {
            self.registers.address += 32;
        } else {
            self.registers.address += 1;
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
