use std::{env, fs, thread};
use std::sync::{Arc, Mutex};
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};
use emulateme::controller::{Controller, ControllerFlags, GenericController, NoController};
use emulateme::cpu::Cpu;
use emulateme::renderer::{NES_HEIGHT, NES_WIDTH, RenderAction, RenderedFrame, Renderer};
use emulateme::rom::parse_rom;
use emulateme::software::SoftwareRenderer;
use crate::streamer::Streamer;
use crate::window::WindowDetails;

mod window;
mod streamer;

#[derive(Clone, Default)]
struct GuiController {
    inner: Arc<Mutex<GenericController>>
}

impl GuiController {
    fn set(&self, flag: ControllerFlags, value: bool) {
        let mut state = self.inner.lock().unwrap();

        state.set(flag, value)
    }
}

impl Controller for GuiController {
    fn read(&mut self) -> u8 {
        let mut state = self.inner.lock().unwrap();

        state.read()
    }
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

        let mut renderer = SoftwareRenderer::new();

        loop {
            cpu.step().unwrap();

            match renderer.render(&mut cpu.memory.ppu, cpu.memory.cycles) {
                RenderAction::None => { },
                RenderAction::SendFrame(frame) => {
                    let mut frame_data = frame_arc.lock().unwrap();

                    *frame_data = Some(frame);

                    window_arc.request_redraw();

                    cpu.interrupt(cpu.vectors.nmi).unwrap()
                }
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
