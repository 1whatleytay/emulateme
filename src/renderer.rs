use crate::ppu::Ppu;


pub const NES_WIDTH: usize = 256;
pub const NES_HEIGHT: usize = 240;

pub const NES_FRAME_SIZE: usize = NES_WIDTH * NES_HEIGHT * 4;

pub struct RenderedFrame {
    pub frame: [u8; NES_FRAME_SIZE]
}

pub enum RenderAction {
    None,
    // Equivalent ot Send NMI
    SendFrame(RenderedFrame)
}

pub trait Renderer {
    fn render(&mut self, ppu: &mut Ppu, cycle: u64) -> RenderAction;
}
