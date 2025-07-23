use std::sync::{Arc, Mutex};
use crate::cpu::CoreMemory;
use crate::ppu::ppu_state::PPUInternalRegisters;
use crate::ppu::{PPURegister, OAM, VRAM};
use crate::ppu::PPURegister::*;
use crate::read_write::ReadWrite;
use crate::read_write::ReadWrite::*;

#[derive(Clone)]
pub struct PPUListener
{
    vram: Arc<Mutex<VRAM>>,
    oam: Arc<Mutex<OAM>>,
    registers: Arc<Mutex<PPUInternalRegisters>>,
}

impl PPUListener
{
    pub fn new(vram: &Arc<Mutex<VRAM>>,
               oam: &Arc<Mutex<OAM>>,
               registers: &Arc<Mutex<PPUInternalRegisters>>) -> PPUListener {
        PPUListener {
            vram: vram.clone(),
            oam: oam.clone(),
            registers: registers.clone(),
        }
    }

    pub fn listen(&self, memory: &CoreMemory, updated_register: PPURegister, read_write: ReadWrite, value: u8) {

        match (updated_register, read_write) {
            (PPUCTRL, WRITE) => {
                /* TODO: Triggered NMI on PPUCTRL write */
            }
            (PPUSTATUS, READ) => {
                self.registers.lock().unwrap().w = 0;
            }
            (OAMADDR, WRITE) => {
                /* TODO: anything here?  */
                /* TODO: OAMADDR should also be set to 0 during pre-render and visible scanlines */
            }
            (OAMDATA, WRITE) => {
                /* TODO: write to the OAM */
                /* TODO: increment OAMADDR */
            }
            (PPUSCROLL, WRITE) => {
                /* TODO */
            }
            (PPUADDR, WRITE) => {
                /* reads the high byte of the address first */
                let write_hi = self.registers.lock().unwrap().w == 0;
                let old_v = self.registers.lock().unwrap().v;
                println!("PPUADDR: writing 0x{:x} with w = {}", value, write_hi);
                self.registers.lock().unwrap().v =
                    if write_hi { (old_v & 0xff) | ((value as u16) << 8 ) }
                    else { value as u16 | (old_v & 0xff00)};
                self.registers.lock().unwrap().w = if write_hi { 1 } else { 0 };
            }
            (PPUDATA, READ) => {
                /* TODO? */
            }
            (PPUDATA, WRITE) => {
                let addr = self.registers.lock().unwrap().v;
                self.vram.lock().unwrap()[addr as usize] = value;
                println!("writing 0x{:x} to VRAM 0x{:x}", value, addr);
                let increase = 1; /* needs to be a function of PPUCTRL */
                self.registers.lock().unwrap().v = addr + increase;
            }
            (OAMDMA, WRITE) => {
                let base_addr = ((value as u16) << 8) as usize;
                memory.copy_range(base_addr, &mut *self.oam.lock().unwrap());
            }
            _ => {}
        }
    }
}