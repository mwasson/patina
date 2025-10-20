use crate::cpu::CoreMemory;
use crate::cpu::MemoryListener;
use crate::ppu::PPURegister::*;
use crate::ppu::{PPURegister, OAM_SIZE, PPU};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct PPUListener {
    ppu: Rc<RefCell<PPU>>,
    read_buffer: u8,
}

impl PPUListener {
    pub fn new(ppu: Rc<RefCell<PPU>>) -> PPUListener {
        PPUListener {
            ppu,
            read_buffer: 0,
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
                PPUCTRL => ppu.ppu_ctrl,
                PPUMASK => ppu.ppu_mask,
                OAMADDR => ppu.oam_addr,
                OAMDATA => ppu.oam[ppu.oam_addr as usize],
                PPUSTATUS => {
                    ppu.internal_regs.w = false;
                    let result = ppu.ppu_status;
                    /* clear vblank flag on read */
                    ppu.ppu_status &= !0x80;
                    result
                }
                PPUADDR => {
                    /* seems like it wouldn't be right? */
                    ppu.internal_regs.v as u8
                }
                PPUDATA => {
                    let result = self.read_buffer;
                    self.read_buffer = ppu.read_vram(ppu.internal_regs.v as usize);

                    ppu.internal_regs.v += if ppu.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };

                    result
                }
                _ => {
                    panic!("unimplemented {:?}", updated_register)
                }
            }
        } else {
            panic!(
                "PPU listener was given non-PPU register address 0x{:?}",
                address
            )
        }
    }

    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8) {
        if let Some(updated_register) = PPURegister::from_addr(address) {
            let mut ppu = self.ppu.borrow_mut();
            match updated_register {
                PPUSTATUS => { /* for now, no effect */ }
                PPUCTRL => {
                    ppu.tall_sprites = value & 0x20 != 0;
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
                    let addr = ppu.oam_addr as usize;
                    ppu.oam[addr] = value;
                    ppu.oam_addr += 1;
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
                        ppu.internal_regs.t =
                            (ppu.internal_regs.t & 0xff) | (((value & 0x3f) as u16) << 8);
                    } else {
                        ppu.internal_regs.t = (ppu.internal_regs.t & 0xff00) | (value as u16);
                        ppu.internal_regs.v = ppu.internal_regs.t;
                    }
                    ppu.internal_regs.w = !ppu.internal_regs.w
                }
                PPUDATA => {
                    let addr = ppu.internal_regs.v as usize;
                    ppu.write_vram(addr, value);
                    ppu.internal_regs.v += if ppu.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                }
                OAMDMA => {
                    let base_addr = (value as u16) << 8;
                    memory.copy_slice(base_addr, OAM_SIZE, &mut ppu.oam);
                }
            }
        }
    }
}
