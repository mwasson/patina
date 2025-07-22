use std::sync::{Arc, Mutex};
use crate::cpu::{MEMORY_SIZE};

/* TODO: should this also handle side-effects? */

pub struct CoreMemory
{
    address_mapper: fn(u16) -> Option<u16>,
    memory: Arc<Mutex<[u8; MEMORY_SIZE]>>
}

impl CoreMemory {
    pub fn write(&mut self, addr: u16, data: u8) {
        self.memory.lock().unwrap()[self.map_address(addr)] = data;
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.memory.lock().unwrap()[self.map_address(addr)]
    }

    fn map_address(&self, addr: u16) -> usize {
        (self.address_mapper)(addr).unwrap_or(addr) as usize
    }

    /* TODO: might want to delete this, this was just to get an idea */
    pub fn clone(&self) -> CoreMemory {
        CoreMemory {
            address_mapper: self.address_mapper,
            memory: Arc::clone(&self.memory)
        }
    }

    pub fn new(memory: [u8; MEMORY_SIZE]) -> CoreMemory {
        CoreMemory {
            address_mapper: CoreMemory::ppu_mirror(),
            memory: Arc::new(Mutex::new(memory))
        }
    }

    /* the PPU registers are at 0x2000 through 0x2007; they're then remapped every eight bytes up
     * through 0x3fff. This reflects that.
     */
    fn ppu_mirror() -> fn(u16) -> Option<u16> {
        |addr:u16| -> Option<u16> {
            if addr <= 0x1fff { /* system memory */
                return Some(addr & 0x7FF);
            }  else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
                return Some(0x2000 | (addr & 0x7));
            } else {
                return None;
            }
        }
    }
}