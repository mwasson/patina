use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::MemoryListener;
use crate::cpu::CoreMemory;
use crate::ppu::PPURegister::*;
use crate::ppu::{PPURegister, PPU, OAM_SIZE};

#[derive(Clone)]
pub struct PPUListener
{
    ppu: Rc<RefCell<PPU>>,
    read_buffer: u8,

    /* while we're prefer for this to not exist and just live in the PPU's t and v registers,
     * this won't work if the scanline all occurs at once; otherwise, the CPU might try to read an
     * address while the PPU is using it for rendering. This separates things out so that can work
     * until PPU::render_scanline() is split up.
     */
    vram_address: u16,
}

impl PPUListener
{
    pub fn new(ppu: Rc<RefCell<PPU>>) -> PPUListener {
        PPUListener {
            ppu,
            read_buffer: 0,
            vram_address: 0,
        }
    }
}

impl MemoryListener for PPUListener {
    fn get_addresses(&self) -> Vec<u16> {
        let mut addrs = Vec::<u16>::new();

        addrs.push(PPURegister::address(&PPUCTRL));
        addrs.push(PPURegister::address(&PPUMASK));
        addrs.push(PPURegister::address(&PPUSTATUS));
        addrs.push(PPURegister::address(&OAMADDR));
        addrs.push(PPURegister::address(&OAMDATA));
        addrs.push(PPURegister::address(&PPUSCROLL));
        addrs.push(PPURegister::address(&PPUADDR));
        addrs.push(PPURegister::address(&PPUDATA));
        addrs.push(PPURegister::address(&OAMDMA));

        addrs
    }

    fn read(&mut self, _memory: &CoreMemory, address: u16) -> u8 {
        if let Some(updated_register) = PPURegister::from_addr(address) {
            let mut ppu = self.ppu.borrow_mut();
            match updated_register {
                PPUCTRL => {
                    ppu.ppu_ctrl
                }
                PPUMASK => {
                    ppu.ppu_mask
                }
                PPUSTATUS => {
                    ppu.internal_regs.w = false;
                    let result = ppu.ppu_status;
                    /* clear vblank flag on read */
                    ppu.ppu_status &= !0x80;
                    result
                }
                PPUADDR => {
                    /* seems like it wouldn't be right? */
                    self.vram_address as u8
                }
                PPUDATA => {
                    let result = self.read_buffer;
                    self.read_buffer = ppu.read_vram(self.vram_address as usize);
                    
                    self.vram_address += if ppu.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };

                    result
                }
                _ => { panic!("unimplemented {:?}", updated_register) }
            }
        } else {
            panic!("PPU listener was given non-PPU register address 0x{:?}", address)
        }
    }

    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8) {
        if let Some(updated_register) = PPURegister::from_addr(address) {
            let mut ppu = self.ppu.borrow_mut();
            match updated_register {
                PPUCTRL => {
                    if value & 0x20 != 0 {
                        panic!("Uh oh, we're in 16 pixel sprite mode..."); /* TODO unimplemented */
                    }
                    ppu.ppu_ctrl = value;
                    ppu.internal_regs.set_nametable_t(value & 0x3);
                    /* TODO writing triggers an immediate NMI when in vblank PPUSTATUS */
                }
                PPUMASK => {
                    ppu.ppu_mask = value;
                }
                OAMADDR => {
                    /* TODO: anything here?  */
                    /* TODO: OAMADDR should also be set to 0 during pre-render and visible scanlines */
                    if value != 0 {
                       panic!("oamaddr not implemented for non-zero values");
                    }
                }
                OAMDATA => {
                    /* TODO: write to the OAM */
                    /* TODO: increment OAMADDR */
                    panic!("oamdata unimplemented");
                }
                PPUSCROLL => {
                    let coarse = (value >> 3) & 0x1f;
                    let fine = value & 0x7;
                    if ppu.internal_regs.is_first_write() {
                        ppu.internal_regs.set_coarse_x_t(coarse);
                        ppu.internal_regs.set_fine_x(fine);
                    } else {
                        ppu.internal_regs.set_coarse_y_t(coarse);
                        ppu.internal_regs.set_fine_y_t(fine);
                    }
                    ppu.internal_regs.w = !ppu.internal_regs.w;
                }
                PPUADDR => {
                    if ppu.internal_regs.is_first_write() {
                        self.vram_address = (self.vram_address & 0xff) | (((value & 0x3f) as u16) << 8);
                    } else {
                        self.vram_address = (self.vram_address & 0xff00) | (value as u16);
                        /* separately, this could affect the PPU's registers--even just clearing
                         * out invalid data, or it could be used to actively update the PPU,
                         * so we also need to send it over there, too
                         */
                        ppu.internal_regs.t = self.vram_address;
                        ppu.internal_regs.v = self.vram_address;

                    }
                    ppu.internal_regs.w = !ppu.internal_regs.w
                }
                PPUDATA => {
                    ppu.write_vram(self.vram_address as usize, value);
                    self.vram_address += if ppu.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                }
                OAMDMA => {
                    let base_addr = (value as u16) << 8;
                    ppu.oam.copy_from_slice(memory.read_slice(base_addr, OAM_SIZE));
                }
                _ => { panic!("unimplemented ppu listener write for {updated_register:?}") }
            }
        }
    }
}