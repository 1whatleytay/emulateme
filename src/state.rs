use serde_derive::{Deserialize, Serialize};
use crate::controller::Controller;
use crate::cpu::{Cpu, Registers, StatusRegister, Vectors};
use crate::memory::Memory;
use crate::ppu::{ControlRegister, MaskRegister, StatusRegister as PpuStatusRegister, NameTable, Palette, PaletteMemory, Ppu, PpuMemory, PpuRegisters, Sprite, RenderRegister};
use crate::rom::Rom;

#[derive(Clone, Serialize, Deserialize)]
pub struct CpuRegisters {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub sp: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateSprite {
    pub y: u8,
    pub number: u8,
    pub mask: u8,
    pub x: u8
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateControlRegister {
    pub increment_32: bool,
    pub base_sprite_pattern_table: bool,
    pub base_background_pattern_table: bool,
    pub sprite_size: bool, // true = 8x16 pixels
    pub ppu_color_ext: bool,
    pub gen_nmi: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateMaskRegister {
    pub greyscale: bool,
    pub show_background_leftmost: bool,
    pub show_sprites_leftmost: bool,
    pub show_background: bool,
    pub show_sprites: bool,
    pub emphasize_red: bool,
    pub emphasize_green: bool,
    pub emphasize_blue: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateStatusRegister {
    pub sprite_hit: bool,
    pub v_blank_hit: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateRenderRegister {
    pub t: u16,
    pub v: u16,
    pub x: u8,
    pub w: bool
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateRegisters {
    pub control: PpuStateControlRegister,
    pub mask: PpuStateMaskRegister,
    pub status: PpuStateStatusRegister,
    pub render: PpuStateRenderRegister,
    pub oam_address: u8,
    pub read_buffer: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateNameTable {
    pub contents: Vec<u8>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStatePaletteMemory {
    pub background_solid: u8,
    pub background: [Palette; 4],
    pub sprite: [Palette; 4],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuStateMemory {
    pub oam: Vec<PpuStateSprite>, // size: 256
    pub names: Vec<PpuStateNameTable>,
    pub palette: PpuStatePaletteMemory, // size: 20
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PpuState {
    pub registers: PpuStateRegisters,
    pub memory: PpuStateMemory,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CpuState {
    pub ram: Vec<u8>, // size: 0x800
    pub controller_cycles: (u64, u64),
    pub registers: CpuRegisters,
    pub ppu: PpuState
}

impl From<&Registers> for CpuRegisters {
    fn from(value: &Registers) -> CpuRegisters {
        CpuRegisters {
            pc: value.pc,
            a: value.a,
            x: value.x,
            y: value.y,
            p: value.p.bits(),
            sp: value.sp,
        }
    }
}

impl From<&CpuRegisters> for Registers {
    fn from(value: &CpuRegisters) -> Registers {
        Registers {
            pc: value.pc,
            a: value.a,
            x: value.x,
            y: value.y,
            p: StatusRegister::from_bits_retain(value.p),
            sp: value.sp,
        }
    }
}

impl From<&PpuStateControlRegister> for ControlRegister {
    fn from(value: &PpuStateControlRegister) -> Self {
        ControlRegister {
            increment_32: value.increment_32,
            base_sprite_pattern_table: value.base_sprite_pattern_table,
            base_background_pattern_table: value.base_background_pattern_table,
            sprite_size: value.sprite_size,
            ppu_color_ext: value.ppu_color_ext,
            gen_nmi: value.gen_nmi,
        }
    }
}

impl From<&ControlRegister> for PpuStateControlRegister {
    fn from(value: &ControlRegister) -> Self {
        PpuStateControlRegister {
            increment_32: value.increment_32,
            base_sprite_pattern_table: value.base_sprite_pattern_table,
            base_background_pattern_table: value.base_background_pattern_table,
            sprite_size: value.sprite_size,
            ppu_color_ext: value.ppu_color_ext,
            gen_nmi: value.gen_nmi,
        }
    }
}

impl From<&PpuStateMaskRegister> for MaskRegister {
    fn from(value: &PpuStateMaskRegister) -> Self {
        MaskRegister {
            greyscale: value.greyscale,
            show_background_leftmost: value.show_background_leftmost,
            show_sprites_leftmost: value.show_sprites_leftmost,
            show_background: value.show_background,
            show_sprites: value.show_sprites,
            emphasize_red: value.emphasize_red,
            emphasize_green: value.emphasize_green,
            emphasize_blue: value.emphasize_blue,
        }
    }
}

impl From<&MaskRegister> for PpuStateMaskRegister {
    fn from(value: &MaskRegister) -> Self {
        PpuStateMaskRegister {
            greyscale: value.greyscale,
            show_background_leftmost: value.show_background_leftmost,
            show_sprites_leftmost: value.show_sprites_leftmost,
            show_background: value.show_background,
            show_sprites: value.show_sprites,
            emphasize_red: value.emphasize_red,
            emphasize_green: value.emphasize_green,
            emphasize_blue: value.emphasize_blue,
        }
    }
}

impl From<&PpuStatusRegister> for PpuStateStatusRegister {
    fn from(value: &PpuStatusRegister) -> Self {
        PpuStateStatusRegister {
            sprite_hit: value.sprite_hit,
            v_blank_hit: value.v_blank_hit,
        }
    }
}


impl From<&PpuStateStatusRegister> for PpuStatusRegister {
    fn from(value: &PpuStateStatusRegister) -> Self {
        PpuStatusRegister {
            sprite_hit: value.sprite_hit,
            v_blank_hit: value.v_blank_hit,
        }
    }
}

impl From<&PpuStateRenderRegister> for RenderRegister {
    fn from(value: &PpuStateRenderRegister) -> Self {
        RenderRegister {
            t: value.t,
            v: value.v,
            x: value.x,
            w: value.w,
        }
    }
}

impl From<&RenderRegister> for PpuStateRenderRegister {
    fn from(value: &RenderRegister) -> Self {
        PpuStateRenderRegister {
            t: value.t,
            v: value.v,
            x: value.x,
            w: value.w,
        }
    }
}

impl From<&PpuRegisters> for PpuStateRegisters {
    fn from(value: &PpuRegisters) -> Self {
        PpuStateRegisters {
            control: (&value.control).into(),
            mask: (&value.mask).into(),
            status: (&value.status).into(),
            render: (&value.render).into(),
            oam_address: value.oam_address,
            read_buffer: value.read_buffer,
        }
    }
}

impl From<&PpuStateRegisters> for PpuRegisters {
    fn from(value: &PpuStateRegisters) -> Self {
        PpuRegisters {
            control: (&value.control).into(),
            mask: (&value.mask).into(),
            status: (&value.status).into(),
            render: (&value.render).into(),
            oam_address: value.oam_address,
            read_buffer: value.read_buffer,
        }
    }
}

impl From<&PpuStateSprite> for Sprite {
    fn from(value: &PpuStateSprite) -> Self {
        Sprite {
            y: value.y,
            number: value.number,
            mask: value.mask,
            x: value.x,
        }
    }
}

impl From<&Sprite> for PpuStateSprite {
    fn from(value: &Sprite) -> Self {
        PpuStateSprite {
            y: value.y,
            number: value.number,
            mask: value.mask,
            x: value.x
        }
    }
}

impl From<&PpuStatePaletteMemory> for PaletteMemory {
    fn from(value: &PpuStatePaletteMemory) -> Self {
        PaletteMemory {
            background_solid: value.background_solid,
            background: value.background,
            sprite: value.sprite,
        }
    }
}

impl From<&PaletteMemory> for PpuStatePaletteMemory {
    fn from(value: &PaletteMemory) -> Self {
        PpuStatePaletteMemory {
            background_solid: value.background_solid,
            background: value.background,
            sprite: value.sprite,
        }
    }
}

impl PpuStateMemory {
    pub fn restore(self, rom: &Rom) -> Option<PpuMemory> {
        Some(PpuMemory {
            rom,
            oam: self.oam.iter().map(Sprite::from)
                .collect::<Vec<Sprite>>().try_into().ok()?,
            names: self.names.into_iter().map(|x| {
                x.contents.try_into().ok().map(|contents| NameTable { contents })
            }).collect::<Option<Vec<NameTable>>>()?.try_into().ok()?,
            palette: (&self.palette).into(),
        })
    }
}

impl<'a> From<&PpuMemory<'a>> for PpuStateMemory {
    fn from(value: &PpuMemory) -> PpuStateMemory {
        PpuStateMemory {
            oam: value.oam.iter()
                .map(|x| x.into())
                .collect(),
            names: value.names.iter()
                .map(|x| PpuStateNameTable { contents: x.contents.to_vec() })
                .collect(),
            palette: (&value.palette).into(),
        }
    }
}

impl<'a, C1: Controller, C2: Controller> From<&Cpu<'a, C1, C2>> for CpuState {
    fn from(value: &Cpu<C1, C2>) -> CpuState {
        CpuState {
            ram: value.memory.ram.to_vec(),
            controller_cycles: value.memory.controller_cycles,
            registers: (&value.registers).into(),
            ppu: PpuState {
                registers: (&value.memory.ppu.registers).into(),
                memory: (&value.memory.ppu.memory).into(),
            },
        }
    }
}

impl CpuState {
    pub fn restore<C1: Controller, C2: Controller>(self, rom: &Rom, controllers: (C1, C2)) -> Option<Cpu<C1, C2>> {
        let mut memory = Memory {
            cycles: 0,
            ram: self.ram.try_into().ok()?,
            rom,
            ppu: Ppu {
                registers: (&self.ppu.registers).into(),
                memory: self.ppu.memory.restore(rom)?,
            },
            saved: [0; 0x2000],
            controllers,
            controller_cycles: self.controller_cycles,
        };

        Some(Cpu {
            vectors: Vectors::new(&mut memory),
            registers: (&self.registers).into(),
            memory,
        })
    }
}
