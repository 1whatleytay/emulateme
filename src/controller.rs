use bitflags::bitflags;

pub trait Controller {
    fn read(&mut self) -> u8;
}

#[derive(Default)]
pub struct NoController;

impl Controller for NoController {
    fn read(&mut self) -> u8 { 0 }
}


#[derive(Default)]
pub struct ControllerFlags(u8);

bitflags! {
    impl ControllerFlags: u8 {
        const A = 0b00000001;
        const B = 0b00000010;
        const SELECT = 0b00000100;
        const START = 0b00001000;
        const UP = 0b00010000;
        const DOWN = 0b00100000;
        const LEFT = 0b01000000;
        const RIGHT = 0b10000000;
    }
}

#[derive(Default)]
pub struct GenericController {
    clock: usize,
    flags: ControllerFlags
}

impl GenericController {
    pub fn press(&mut self, flags: ControllerFlags) {
        self.flags = flags
    }

    pub fn set(&mut self, flag: ControllerFlags, value: bool) {
        self.flags.set(flag, value)
    }
}

impl Controller for GenericController {
    fn read(&mut self) -> u8 {
        let clock = self.clock % 8;

        let value = self.flags.0 & (1 << clock) != 0;

        self.clock += 1;

        if value { 1 } else { 0 }
    }
}
