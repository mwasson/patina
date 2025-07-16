use std::collections::HashMap;
use std::iter::Map;
use crate::cpu::MEMORY_SIZE;

struct MemoryMirror
{
    address_mapper: fn(u16) -> Option<u16>
}

impl MemoryMirror {
    pub fn write(&self, memory: &mut [u8; MEMORY_SIZE],  addr: u16, data: u8) {
        memory[self.map_address(addr)] = data;
    }

    pub fn read(&self, memory: &[u8; MEMORY_SIZE], addr: u16) -> u8 {
        memory[self.map_address(addr)]
    }

    fn map_address(&self, addr: u16) -> usize {
        (self.address_mapper)(addr).unwrap_or(addr) as usize
    }

    fn new(address_mapper: fn(u16) -> Option<u16>) -> MemoryMirror {
        MemoryMirror {
            address_mapper
        }
    }

    /* the PPU registers are at 0x2000 through 0x2007; they're then remapped every eight bytes up
     * through 0x3fff. This reflects that.
     */
    fn ppu_mirror() -> MemoryMirror {
        let address_mapper = |addr:u16| -> Option<u16> {
            if addr <= 0x1fff { /* system memory */
                return Some(addr & 0x7FF);
            }  else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
                return Some(0x2000 | (addr & 0x7));
            } else {
                return None;
            }
        };

        MemoryMirror::new(address_mapper)
    }
}