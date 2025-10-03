use crate::mapper::Mapper;
use crate::ppu::{NametableMirroring, Tile};
use crate::rom::Rom;

const PRG_BANK_SIZE: usize = 1 << 15;
const CHR_BANK_SIZE: usize = 1 << 13; /* 8kb CHR RAM */

pub struct NROM {
    prg_ram: Box<[u8; PRG_BANK_SIZE]>,
    chr_ram: Box<[u8; CHR_BANK_SIZE]>,
    is_32_kb: bool, /* NROM can be either 32kb or 16kb mirrored */
    nametable_mirroring: NametableMirroring,
}

impl NROM {
    pub fn new(rom: &Rom) -> NROM {
        let mut prg_ram = Box::new([0; PRG_BANK_SIZE]);
        /* TODO lol bad */
        let is_32_kb = rom.prg_data.len() == PRG_BANK_SIZE;
        prg_ram[0..rom.prg_data.len()].copy_from_slice(&*rom.prg_data);

        let mut chr_ram = Box::new([0; CHR_BANK_SIZE]);
        chr_ram[0x0000..rom.chr_data.len()].copy_from_slice(&rom.chr_data);

        NROM {
            prg_ram,
            chr_ram,
            is_32_kb,
            nametable_mirroring: rom.nametable_mirroring()
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
    fn read_prg(&self, address: u16) -> u8 {
        self.prg_ram[self.map_address(address)]
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        let mapped_address = self.map_address(address);
        &self.prg_ram[mapped_address..mapped_address + size]
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        panic!("NROM: ATTEMPTED TO WRITE TO PRG-ROM ADDRESS 0x{address:x} VALUE 0x{value:x}");
    }

    fn read_chr(&self, address: u16) -> u8 {
        self.chr_ram[address as usize]
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        panic!("NROM: ATTEMPTED TO WRITE TO CHR-ROM ADDRESS 0x{address:x} VALUE 0x{value:x}");
    }
    
    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.nametable_mirroring.clone()
    }
}