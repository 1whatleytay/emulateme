use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use crate::ppu::{Ppu, PpuMemoryError};
use crate::rom::Rom;

#[derive(Clone, Debug)]
pub enum MemoryError {
    UnmappedRead(u16),
    UnmappedWrite(u16),
    PpuError(PpuMemoryError)
}

pub trait Controller {
    fn read(&mut self) -> u8;
}

#[derive(Default)]
pub struct NoController;

impl Controller for NoController {
    fn read(&mut self) -> u8 { 0 }
}

pub struct Memory<'a, C1: Controller, C2: Controller> {
    pub cycles: u64,
    pub ram: [u8; 0x800],
    pub rom: &'a Rom,
    pub ppu: Ppu<'a>,
    pub saved: [u8; 0x2000], // 0x6000
    pub controllers: (C1, C2),
}

impl From<PpuMemoryError> for MemoryError {
    fn from(value: PpuMemoryError) -> Self {
        MemoryError::PpuError(value)
    }
}

impl Display for MemoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::UnmappedRead(address) =>
                write!(f, "Unmapped read to ${address:04X}"),
            MemoryError::UnmappedWrite(address) =>
                write!(f, "Unmapped write to ${address:04X}"),
            MemoryError::PpuError(error) =>
                Display::fmt(error, f)
        }
    }
}

impl Error for MemoryError { }

impl<'a, C1: Controller, C2: Controller> Memory<'a, C1, C2> {
    pub fn cycle(&mut self) {
        self.cycles += 1;
    }

    pub fn cycle_many(&mut self, times: u64) {
        self.cycles += times;
    }

    fn oam_dma(&mut self, page: u8) -> Result<(), MemoryError> {
        let base_address = (page as u16) << 8;
        let mut oam = [0u8; 256];

        for (i, o) in oam.iter_mut().enumerate() {
            let value = self.get(base_address + i as u16)?;

            self.cycle();

            *o = value;
        }

        self.ppu.replace_oam(oam);
        self.cycle(); // wait cycle

        Ok(())
    }

    pub fn pass_get(&mut self, address: u16) -> Result<u8, MemoryError> {
        Ok(match address {
            0..=0x1fff => {
                let target = (address % 0x800) as usize;

                self.ram[target]
            },
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data()?,
            0x4015 => 0, // APU Status
            0x4016 => self.controllers.0.read(), // Controller 1
            0x4017 => self.controllers.1.read(), // Controller 2
            0x6000..=0x7FFF => {
                let target = (address - 0x6000) as usize;

                self.saved[target]
            },
            0x8000..=0xffff => {
                let target = (address - 0x8000) as usize % self.rom.prg_rom.len();

                self.rom.prg_rom[target]
            },
            _ => return Err(MemoryError::UnmappedRead(address))
        })
    }

    pub fn get(&mut self, address: u16) -> Result<u8, MemoryError> {
        self.cycle();

        self.pass_get(address)
    }

    pub fn pass_set(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        match address {
            0..=0x1fff => {
                let target = (address % 0x800) as usize;

                self.ram[target] = value
            },
            0x2000 => self.ppu.write_ctrl(value),
            0x2001 => self.ppu.write_mask(value),
            0x2003 => self.ppu.write_oam_address(value),
            0x2004 => self.ppu.write_oam_data(value),
            0x2005 => self.ppu.write_scroll(value),
            0x2006 => self.ppu.write_address(value),
            0x2007 => self.ppu.write_data(value)?,
            0x4000..=0x4013 => (), // APU
            0x4014 => self.oam_dma(value)?,
            0x4015 => (), // APU Status
            0x4016 => (), // Controller
            0x4017 => (), // APU Frame Counter
            0x6000..=0x7FFF => {
                let target = (address - 0x6000) as usize;

                self.saved[target] = value
            }
            _ => return Err(MemoryError::UnmappedWrite(address))
        }

        Ok(())
    }

    pub fn set(&mut self, address: u16, value: u8) -> Result<(), MemoryError> {
        self.cycle();

        self.pass_set(address, value)
    }
    
    pub fn get_short(&mut self, address: u16) -> Result<u16, MemoryError> {
        let low = self.get(address)? as u16;
        let high = self.get(address.wrapping_add(1))? as u16;
        
        Ok((high << 8) | low)
    }

    /*
    pub fn set_short(&mut self, address: u16, value: u16) -> Result<(), MemoryError> {
        let low = (value & 0xFF) as u8;
        let high = (value >> 8) as u8;
        
        self.set(address, low)?;
        self.set(address.wrapping_add(1), high)
    }
    */

    pub fn new(rom: &'a Rom, controllers: (C1, C2)) -> Memory<'a, C1, C2> {
        Memory {
            cycles: 0,
            ram: [0; 0x800],
            ppu: Ppu::new(rom),
            rom,
            saved: [0; 0x2000],
            controllers,
        }
    }
}
