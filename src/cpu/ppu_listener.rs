use std::sync::mpsc::Sender;
use crate::cpu::{CoreMemory};
use crate::ppu::{PPURegister, PPUState, OAM_SIZE, PPU_MEMORY_SIZE, VRAM};
use crate::cpu::cpu_to_ppu_message::CpuToPpuMessage;
use crate::cpu::cpu_to_ppu_message::CpuToPpuMessage::{Memory, Oam, ScrollX, ScrollY};
use crate::ppu::PPURegister::*;
use crate::rom::Rom;

#[derive(Clone)]
pub struct PPUListener
{
    update_sender: Sender<CpuToPpuMessage>,
    vram_addr: usize, /* vram address to read/write */
    first_write: bool, /* for dual-write mapped registers */
    read_buffer: u8,
    ppu_ctrl: u8,   /* ppu write register, controlled by cpu */
    ppu_mask: u8,   /* ppu write register, controlled by cpu */
    pub ppu_status: u8, /* ppu read register, controlled by ppu, handled in updates */
    local_vram_copy: VRAM /* for easy reads from PPUDATA, since only the CPU writes to it */
}

impl PPUListener
{
    pub fn new(rom: &Rom, update_sender: Sender<CpuToPpuMessage>) -> PPUListener {
        let mut local_vram_copy =[0; PPU_MEMORY_SIZE];
        local_vram_copy[0x0000..rom.chr_data.len()].copy_from_slice(&rom.chr_data);
        PPUListener {
            update_sender,
            vram_addr: 0, /* vram address to read/write */
            first_write: true, /* for dual-write mapped registers */
            read_buffer: 0,
            ppu_ctrl: 0,
            ppu_mask: 0,
            ppu_status: 0,
            local_vram_copy,
        }
    }

    pub fn listen_write(&mut self, memory: &CoreMemory, updated_register: &PPURegister, value: u8) {
        match updated_register {
            PPUCTRL => {
                if value & 0x20 != 0 {
                    panic!("Uh oh, we're in 16 pixel sprite mode..."); /* TODO unimplemented */
                }
                /* TODO: effects on scroll controls */
                self.ppu_ctrl = value;
                self.send_update(CpuToPpuMessage::PpuCtrl(value)); /* includes nametable update */
                /* TODO writing triggers an immediate NMI when in vblank PPUSTATUS */
            }
            PPUMASK => {
                self.send_update(CpuToPpuMessage::PpuMask(value));
            }
            OAMADDR => {
                /* TODO: anything here?  */
                /* TODO: OAMADDR should also be set to 0 during pre-render and visible scanlines */
            }
            OAMDATA => {
                /* TODO: write to the OAM */
                /* TODO: increment OAMADDR */
            }
            PPUSCROLL => {
                if self.first_write {
                    self.send_update(ScrollX((value >> 3) & 0x1f, value & 0x7));
                } else {
                    self.send_update(ScrollY((value >> 3) & 0x1f, value & 0x7));
                }
                self.first_write = !self.first_write;
            }
            PPUADDR => {
                /* writes high byte first */
                self.vram_addr =
                    if self.first_write { (self.vram_addr & 0xff) | ((0x3f & value as usize) << 8 ) }
                    else { value as usize | (self.vram_addr & 0xff00)};
                /* TODO HACK REMOVE */
                if(self.first_write) {
                    self.send_update(CpuToPpuMessage::PpuCtrl((self.ppu_ctrl & !3) | ((value & 0xa) >> 2)))
                }
                self.first_write = !self.first_write;
            }
            PPUDATA => {
                let addr = PPUState::vram_address_mirror(self.vram_addr);
                /* send a message to the PPU to update */
                self.send_update(Memory(addr, value));
                /* and make a local copy, in case the program reads PPUDATA */
                self.local_vram_copy[addr] = value;
                let increase = if self.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                self.vram_addr += increase;
            }
            OAMDMA => {
                let base_addr = ((value as u16) << 8) as usize;
                let mut copied_block: [u8; OAM_SIZE] = [0; OAM_SIZE];
                copied_block.copy_from_slice(&memory[base_addr..base_addr + OAM_SIZE]);
                self.send_update(Oam(copied_block));
            }
            _ => { panic!("unimplemented") }
        }
    }

    pub fn listen_read(&mut self, updated_register: &PPURegister) -> u8 {
        match updated_register {
            PPUCTRL => {
                self.ppu_ctrl
            }
            PPUMASK => {
                self.ppu_mask
            }
            PPUSTATUS => {
                self.first_write = true;
                let result = self.ppu_status;
                self.ppu_status &= !0x80;
                result
            }
            PPUDATA => {
                let new_buffered_val = self.local_vram_copy[PPUState::vram_address_mirror(self.vram_addr)];

                let result = self.read_buffer;
                self.read_buffer = new_buffered_val;

                let increase = if self.ppu_ctrl & 0x4 != 0 { 32 } else { 1 };
                self.vram_addr += increase;

                result
            }
            _ => { panic!("unimplemented") }
        }
    }

    fn send_update(&self, update: CpuToPpuMessage) {
        self.update_sender.send(update).expect("");
    }
}