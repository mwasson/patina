use crate::mapper::Mapper;
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

pub struct UxROM {
    chr: Vec<u8>,
    prg_rom: Vec<u8>,
    nametable_mirroring: NametableMirroring,
    lo_bank: u8,
}

impl UxROM {
    pub fn new(rom: &Rom) -> Self {
        let chr = if rom.chr_data.len() > 0 {
            rom.chr_data.clone()
        } else {
            vec![0; 1 << 13]
        };

        UxROM {
            chr,
            prg_rom: rom.prg_data.clone(),
            nametable_mirroring: rom.nametable_mirroring(),
            lo_bank: 0,
        }
    }

    fn map_prg_addr(&self, address: u16) -> usize {
        if address < 0x8000 {
            panic!("addresses below 0x8000 not used in UxROM");
        }

        if address < 0xC000 {
            self.lo_bank as usize * (1 << 14) + address as usize - 0x8000
        } else {
            self.prg_rom.len() - (1 << 14) + address as usize - 0xc000
        }
    }
}

impl Mapper for UxROM {
    fn read_prg(&self, address: u16) -> u8 {
        if address < 0x8000 {
            panic!("addresses below 0x8000 not used in UxROM");
        }

        self.prg_rom[self.map_prg_addr(address)]
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        let mapped_addr = self.map_prg_addr(address);

        &self.prg_rom[mapped_addr..mapped_addr + size]
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        if address < 0x8000 {
            panic!("addresses below 0x8000 not used in UxROM");
        } else {
            self.lo_bank = value & 0xf; /* TODO: should be 0x7 for some variants */
        }
    }

    fn read_chr(&self, address: u16) -> u8 {
        self.chr[address as usize]
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        self.chr[address as usize] = value;
        // panic!("cannot write to CHR-ROM in UxROM")
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.nametable_mirroring.clone()
    }
}
