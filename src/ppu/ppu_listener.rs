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
    vram_addr: usize, /* vram address to read/write */
    first_write: bool, /* for dual-write mapped registers */
    read_buffer: u8,
}

impl PPUListener
{
    pub fn new(ppu: Rc<RefCell<PPU>>) -> PPUListener {
        PPUListener {
            ppu,
            vram_addr: 0, /* vram address to read/write */
            first_write: true, /* for dual-write mapped registers */
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
            match updated_register {
                PPUCTRL => {
                    self.ppu.borrow_mut().ppu_ctrl
                }
                PPUMASK => {
                    self.ppu.borrow_mut().ppu_mask
                }
                PPUSTATUS => {
                    let mut unwrapped_ppu = self.ppu.borrow_mut();
                    self.first_write = true;
                    let result = unwrapped_ppu.ppu_status;
                    /* clear vblank flag on read */
                    unwrapped_ppu.ppu_status &= !0x80;
                    result
                }
                PPUADDR => {
                    self.vram_addr as u8
                }
                PPUDATA => {
                    let ppu_unwrapped = self.ppu.borrow();
                    let new_buffered_val = ppu_unwrapped.read_vram(self.vram_addr);

                    let result = self.read_buffer;
                    self.read_buffer = new_buffered_val;

                    let increase = if self.ppu.borrow().ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                    self.vram_addr += increase;

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
            let mut ppu_unwrapped = self.ppu.borrow_mut();
            match updated_register {
                PPUCTRL => {
                    if value & 0x20 != 0 {
                        panic!("Uh oh, we're in 16 pixel sprite mode..."); /* TODO unimplemented */
                    }
                    ppu_unwrapped.ppu_ctrl = value;
                    ppu_unwrapped.scroll.nametable = value & 0x3;
                    /* TODO writing triggers an immediate NMI when in vblank PPUSTATUS */
                }
                PPUMASK => {
                    ppu_unwrapped.ppu_mask = value;
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
                    if self.first_write {
                        ppu_unwrapped.scroll.coarse_x = coarse;
                        ppu_unwrapped.scroll.fine_x = fine;
                    } else {
                        ppu_unwrapped.scroll.coarse_y = coarse;
                        ppu_unwrapped.scroll.fine_y = fine;
                    }
                    self.first_write = !self.first_write;
                }
                PPUADDR => {
                    /* writes high byte first */
                    self.vram_addr =
                        if self.first_write { (self.vram_addr & 0xff) | ((0x3f & value as usize) << 8) } else { value as usize | (self.vram_addr & 0xff00) };
                    /* TODO HACK REMOVE */
                    if self.first_write {
                        let new_ctrl = (ppu_unwrapped.ppu_ctrl & !3) | ((value & 0xa) >> 2);
                        ppu_unwrapped.ppu_ctrl = new_ctrl;
                        ppu_unwrapped.scroll.nametable = new_ctrl & 0x3;
                    }
                    self.first_write = !self.first_write;
                }
                PPUDATA => {
                    /* send a message to the PPU to update */
                    ppu_unwrapped.write_vram(self.vram_addr, value);
                    let increase = if ppu_unwrapped.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                    self.vram_addr += increase;
                }
                OAMDMA => {
                    let base_addr = (value as u16) << 8;
                    ppu_unwrapped.oam.copy_from_slice(memory.read_slice(base_addr, OAM_SIZE));
                }
                _ => { panic!("unimplemented") }
            }
        }
    }
}