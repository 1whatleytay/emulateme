use std::{env, fs, thread};
use std::sync::{Arc, Mutex};
use bitflags::bitflags;
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};
use emulateme::cpu::Cpu;
use emulateme::decoder::{Decoder, decoder_iterator};
use emulateme::disassembler::Disassembler;
use emulateme::memory::{Controller, NoController};
use emulateme::renderer::{RenderAction, Renderer};
use emulateme::rom::parse_rom;
use emulateme::software::{NES_HEIGHT, NES_WIDTH, RenderedFrame, SoftwareRenderer};
use crate::streamer::Streamer;
use crate::window::WindowDetails;

mod window;
mod streamer;

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
struct GuiControllerState {
    clock: usize,
    flags: ControllerFlags
}

#[derive(Clone, Default)]
struct GuiController {
    inner: Arc<Mutex<GuiControllerState>>
}

impl GuiController {
    fn set(&self, flag: ControllerFlags, value: bool) {
        let mut state = self.inner.lock().unwrap();

        state.flags.set(flag, value)
    }
}

impl Controller for GuiController {
    fn read(&mut self) -> u8 {
        let mut state = self.inner.lock().unwrap();
        let clock = state.clock % 8;

        let value = state.flags.0 & (1 << clock) != 0;

        state.clock += 1;

        if value { 1 } else { 0 }
    }
}

fn decode_state<C1: Controller, C2: Controller, F: FnMut(RenderedFrame)>(cpu: &mut Cpu<C1, C2>, renderer: &SoftwareRenderer<F>) -> String {
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
            A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{} PPU: {}, {}",
                       registers.pc, registers.a, registers.x, registers.y,
                       registers.p.bits(), registers.sp, cycles, renderer.scan_x, renderer.scan_y
    )
}

fn main() {
    let arguments = env::args().collect::<Vec<String>>();

    let Some(path) = arguments.get(1) else {
        panic!("Usage: emgui /path/to/rom.nes")
    };

    let rom_bytes = fs::read(path).unwrap();
    let (_, rom) = parse_rom(&rom_bytes).unwrap();

    if rom.chr_rom.is_empty() {
        panic!("ROM has no CHR/Graphics data, it will probably crash the renderer, aborting.")
    }

    let (window, event_loop) = WindowDetails::make("EmulateMe Gui").unwrap();

    let streamer = Streamer::new(&window.details, NES_WIDTH, NES_HEIGHT);

    let frame_data = Arc::new(Mutex::new(Some(RenderedFrame::default())));

    let window_arc = window.window.clone();
    let frame_arc = frame_data.clone();

    let controller = GuiController::default();
    let controller_copy = controller.clone();

    thread::spawn(move || {
        let mut cpu = Cpu::new(&rom, None, (controller_copy, NoController));

        let mut renderer = SoftwareRenderer::new(|frame| {
            let mut frame_data = frame_arc.lock().unwrap();

            *frame_data = Some(frame);

            window_arc.request_redraw();
        });

        loop {
            // if cpu.registers.pc != 0x8057 {
            //     println!("{}", decode_state(&mut cpu, &renderer));
            // }

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
    }, |event| {
        let value = event.state == ElementState::Pressed;

        match event.physical_key {
            PhysicalKey::Code(KeyCode::KeyX) => controller.set(ControllerFlags::A, value),
            PhysicalKey::Code(KeyCode::KeyZ) => controller.set(ControllerFlags::B, value),
            PhysicalKey::Code(KeyCode::ArrowUp) => controller.set(ControllerFlags::UP, value),
            PhysicalKey::Code(KeyCode::ArrowDown) => controller.set(ControllerFlags::DOWN, value),
            PhysicalKey::Code(KeyCode::ArrowLeft) => controller.set(ControllerFlags::LEFT, value),
            PhysicalKey::Code(KeyCode::ArrowRight) => controller.set(ControllerFlags::RIGHT, value),
            PhysicalKey::Code(KeyCode::Enter) => controller.set(ControllerFlags::SELECT, value),
            PhysicalKey::Code(KeyCode::KeyL) => controller.set(ControllerFlags::START, value),

            _ => { }
        }
    }).unwrap();
}
