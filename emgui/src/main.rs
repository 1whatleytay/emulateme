use std::{env, fs, thread};
use std::sync::{Arc, Mutex};
use emulateme::cpu::Cpu;
use emulateme::decoder::{Decoder, decoder_iterator};
use emulateme::disassembler::Disassembler;
use emulateme::renderer::{RenderAction, Renderer};
use emulateme::rom::parse_rom;
use emulateme::software::{NES_HEIGHT, NES_WIDTH, RenderedFrame, SoftwareRenderer};
use crate::streamer::Streamer;
use crate::window::WindowDetails;

mod window;
mod streamer;

fn decode_state(cpu: &mut Cpu) -> String {
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
    let arguments = env::args().collect::<Vec<String>>();

    let Some(path) = arguments.get(1) else {
        panic!("Usage: emgui /path/to/rom.nes")
    };

    let rom_bytes = fs::read(path).unwrap();
    let (_, rom) = parse_rom(&rom_bytes).unwrap();

    let (window, event_loop) = WindowDetails::make("EmulateMe Gui").unwrap();

    let streamer = Streamer::new(&window.details, NES_WIDTH, NES_HEIGHT);

    let frame_data = Arc::new(Mutex::new(Some(RenderedFrame::default())));

    let window_arc = window.window.clone();
    let frame_arc = frame_data.clone();

    thread::spawn(move || {
        let mut cpu = Cpu::new(&rom, None);

        let mut renderer = SoftwareRenderer::new(|frame| {
            let mut frame_data = frame_arc.lock().unwrap();

            *frame_data = Some(frame);

            window_arc.request_redraw();
        });

        loop {
            println!("{}", decode_state(&mut cpu));

            cpu.step().unwrap();

            match renderer.render(&mut cpu.memory.ppu, cpu.memory.cycles) {
                RenderAction::None => { },
                RenderAction::SendNMI => cpu.interrupt(cpu.vectors.nmi).unwrap()
            }
        }
    });

    window.run(event_loop, || {
        let Ok(mut frame) = frame_data.try_lock() else {
            window.window.request_redraw();

            return;
        };

        if let Some(frame) = frame.take() {
            streamer.render_frame(&frame.frame, window.size.get()).unwrap();
        } else {
            streamer.redraw_frame(window.size.get()).unwrap();
        }
    }).unwrap();
}
