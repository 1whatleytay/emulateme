use crate::ppu::Ppu;

pub enum RenderAction {
    None,
    SendNMI
}

pub trait Renderer {
    fn render(&mut self, ppu: &mut Ppu, cycle: u64) -> RenderAction;
}
