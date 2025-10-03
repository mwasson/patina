use crate::mapper::Mapper;
use crate::rom::Rom;

const PRG_BANK_SIZE: usize = 1 << 15;

pub struct NROM {
    memory: Box<[u8; PRG_BANK_SIZE]>, /* TODO fix */
    is_32_kb: bool, /* NROM can be either 32kb or 16kb mirrored */
}

impl NROM {
    pub fn new(rom: &Rom) -> NROM {
        let mut memory = Box::new([0; PRG_BANK_SIZE]);
        /* TODO lol bad */
        let is_32_kb = rom.prg_data.len() == PRG_BANK_SIZE;
        memory[0..rom.prg_data.len()].copy_from_slice(&*rom.prg_data);

        NROM {
            memory,
            is_32_kb,
        }
    }

    fn map_address(&self, address: u16) -> usize {
        /* mapper is assigned 0x8000 to 0xffff; subtract 0x8000 to turn into an array index */
        if self.is_32_kb {
            (address - 0x8000) as usize
        /* if there's 16 kb of ram, 0xc000-0xffff are mirrors of 0x8000-0xbffff */
        } else {
            ((address & !0x4000) - 0x8000) as usize
        }
    }
}

impl Mapper for NROM {
    fn read(&self, address: u16) -> u8 {
        self.memory[self.map_address(address)]
    }

    fn read_slice(&self, address: u16, size: usize) -> &[u8] {
        let mapped_address = self.map_address(address);
        &self.memory[mapped_address..mapped_address + size]
    }

    fn write(&mut self, address: u16, value: u8) {
        panic!("NROM: ATTEMPTED TO WRITE TO PRG-ROM ADDRESS 0x{address:x} VALUE 0x{value:x}");
    }
}