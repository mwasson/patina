use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use crate::ppu::ppu_state::PPUInternalRegisters;
use crate::ppu::PPURegister;

#[derive(Clone)]
pub struct PPUListener
{
    vram: Arc<Mutex<[u8; crate::ppu::ppu_state::PPU_MEMORY_SIZE]>>,
    ppu_internal_registers: Arc<Mutex<PPUInternalRegisters>>,
}

impl PPUListener
{
    pub fn new(vram: &Arc<Mutex<[u8; crate::ppu::ppu_state::PPU_MEMORY_SIZE]>>,
               registers: &Arc<Mutex<PPUInternalRegisters>>) -> PPUListener {
        PPUListener {
            vram: vram.clone(),
            ppu_internal_registers: registers.clone()
        }
    }

    pub fn listen(&self, updated_register: PPURegister) {
        println!("PPU LISTENER CALLED ON PPU REGISTER {:?}", updated_register);
    }
}