use bitflags::bitflags;
use crate::controller::Controller;
use crate::memory::Memory;
use crate::rom::Rom;

#[derive(Clone)]
pub struct StatusRegister(u8);

bitflags! {
    impl StatusRegister: u8 {
        const CARRY = 0b00000001;
        const ZERO = 0b00000010;
        const INTERUPT = 0b00000100;
        const DECIMAL = 0b00001000;
        const BREAK = 0b00010000;
        const ENABLED = 0b00100000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = 0b10000000;
    }
}

#[derive(Clone)]
pub struct Registers {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: StatusRegister,
    pub sp: u8,
}

pub struct Vectors {
    pub nmi: u16,
    pub reset: u16,
    pub interrupt: u16
}

pub struct Cpu<'a, C1: Controller, C2: Controller> {
    pub vectors: Vectors,
    pub registers: Registers,
    pub memory: Memory<'a, C1, C2>
}

impl Registers {
    pub fn new(pc: u16) -> Registers {
        Registers {
            pc,
            a: 0,
            x: 0,
            y: 0,
            p: StatusRegister::ENABLED | StatusRegister::INTERUPT,
            sp: 0xFD,
        }
    }
}

impl Vectors {
    pub fn new<C1: Controller, C2: Controller>(memory: &mut Memory<C1, C2>) -> Vectors {
        const DEFAULT: u16 = 0x8000;

        Vectors {
            nmi: memory.get_short(0xFFFA).unwrap_or(DEFAULT),
            reset: memory.get_short(0xFFFC).unwrap_or(DEFAULT),
            interrupt: memory.get_short(0xFFFE).unwrap_or(DEFAULT),
        }
    }
}

impl<'a, C1: Controller, C2: Controller> Cpu<'a, C1, C2> {
    pub fn new(rom: &'a Rom, pc: Option<u16>, controllers: (C1, C2)) -> Cpu<'a, C1, C2> {
        let mut memory = Memory::new(rom, controllers);

        memory.cycle();

        let vectors = Vectors::new(&mut memory);

        Cpu {
            registers: Registers::new(pc.unwrap_or(vectors.reset)),
            vectors,
            memory
        }
    }
}
