use std::sync::{Arc, Mutex};
use crate::cpu::{CoreMemory, ProgramState};
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
    memory: Arc<CoreMemory>,
}

impl PPUListener
{
    pub fn new(vram: &Arc<Mutex<VRAM>>,
               oam: &Arc<Mutex<OAM>>,
               registers: &Arc<Mutex<PPUInternalRegisters>>,
               memory: &CoreMemory) -> PPUListener {
        PPUListener {
            vram: vram.clone(),
            oam: oam.clone(),
            registers: registers.clone(),
            memory: Arc::new(memory.clone()),
        }
    }

    pub fn listen(&self, memory: &CoreMemory, updated_register: PPURegister, read_write: ReadWrite, value: u8) -> Option<u8> {

        let mut result = None;

        match (updated_register, read_write) {
            (PPUCTRL, WRITE) => {
                if value & 0x20 != 0 {
                    panic!("Uh oh, we're in 16 pixel sprite mode...");
                }
                /* TODO writing triggers an immediate NMI when in vblank PPUSTATUS */
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
                let is_first_right = self.registers.lock().unwrap().w == 0;
                let t = self.registers.lock().unwrap().t;

                if is_first_right {
                    /* top five bits of the input become the bottom five bits of t */
                    self.registers.lock().unwrap().t = (t & !0x1f) | ((value as u16 & 0x1f) >> 3) ;
                    self.registers.lock().unwrap().x = value & 0x7; /* bottom three bits */;
                    self.registers.lock().unwrap().w = 1;
                /* second write */
                } else {
                    self.registers.lock().unwrap().t = 0;
                    self.registers.lock().unwrap().w = 0;
                }
            }
            (PPUADDR, WRITE) => {
                /* reads the high byte of the address first */
                let write_hi = self.registers.lock().unwrap().w == 0;
                let old_v = self.registers.lock().unwrap().v;
                self.registers.lock().unwrap().v =
                    if write_hi { (old_v & 0xff) | ((value as u16) << 8 ) }
                    else { value as u16 | (old_v & 0xff00)};
                self.registers.lock().unwrap().w = if write_hi { 1 } else { 0 };
            }
            (PPUDATA, READ) => {
                /* TODO: in actuality, this reads from an internal buffer, not VRAM directly;
                 * for now assuming I can get away without that but it's probably necessary for
                 * complete fidelity.
                 */
                let vram_loc = self.registers.lock().unwrap().v;
                let new_buffered_val = self.vram.lock().unwrap()[vram_loc as usize];

                result = Some(self.registers.lock().unwrap().read_buffer);
                self.registers.lock().unwrap().read_buffer = new_buffered_val;

                let increase = if PPUCTRL.read(memory) & 0x4 != 0 { 32 } else { 1 };
                self.registers.lock().unwrap().v = vram_loc + increase;
            }
            (PPUDATA, WRITE) => {
                let addr = (self.registers.lock().unwrap().v & 0x3fff) as usize;

                self.vram.lock().unwrap()[addr] = value;
                let increase = if PPUCTRL.read(memory) & 0x4 != 0 { 32 } else { 1 };
                self.registers.lock().unwrap().v = addr as u16 + increase;

                if addr == 0x23f5 && value == 0xaa {
                    println!("VRAM write: [0x{addr:x}] = 0x{value:x}");
                    // self.memory.print_memory(36539 - 36539 % 0x100, 0x100);
                }
            }
            (OAMDMA, WRITE) => {
                let base_addr = ((value as u16) << 8) as usize;
                memory.copy_range(base_addr, &mut *self.oam.lock().unwrap());
            }
            _ => {}
        }

        result
    }
}