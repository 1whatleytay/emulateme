use std::{env, fs};
use emulateme::cpu::Cpu;
use emulateme::decoder::{Decoder, decoder_iterator};
use emulateme::disassembler::Disassembler;
use emulateme::memory::Controller;
use emulateme::rom::parse_rom;

fn decode_state<C1: Controller, C2: Controller>(cpu: &mut Cpu<C1, C2>) -> String {
    let registers = cpu.registers.clone();
    let cycles = cpu.memory.cycles;

    let mut size: u16 = 0;

    let next = decoder_iterator(|i| {
        size += 1;

        cpu.memory.pass_get(registers.pc + i).ok()
    });

    let Some(instruction) = Disassembler { pc: registers.pc }.decode(next) else {
        match cpu.memory.pass_get(registers.pc) {
            Ok(op) =>
                panic!("Unable to decode op {:02X} at PC: {:04X}", op, registers.pc),
            Err(err) =>
                panic!("Unable to access instruction at PC: {:04X} with error: {}", registers.pc, err)
        }
    };

    let components = (0..size).map(|i| {
        cpu.memory.pass_get(registers.pc + i)
            .map(|x| format!("{x:02X}"))
            .unwrap_or_else(|_| "RR".to_string())
    }).collect::<Vec<String>>().join(" ");

    format!("{:04X}  {components:8 }  {instruction:30 }  \
            A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            registers.pc, registers.a, registers.x, registers.y,
            registers.p.bits(), registers.sp, cycles
    )
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    let path = arguments.get(1)
        .expect("Requires one argument, a path to a valid NES ROM.");

    let bytes = fs::read(path)
        .unwrap_or_else(|_| panic!("Cannot find ROM at path {path}"));

    let (_, rom) = parse_rom(&bytes)
        .unwrap_or_else(|_| panic!("Failed to parse ROM contents at path {path}"));

    let mut cpu = Cpu::new(&rom, Some(0xC000), );

    loop {
        println!("{}", decode_state(&mut cpu));

        cpu.step().unwrap();
    }
}
