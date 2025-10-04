use crate::mapper::bank_array::BankArray;
use crate::mapper::{Mapper, SIZE_16_KB, SIZE_8_KB};
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

pub struct UxROM {
    prg_banks: BankArray,
    chr_bank: BankArray,
    nametable_mirroring: NametableMirroring,
}

impl UxROM {
    pub fn new(rom: &Rom) -> Self {
        let mut chr_bank = BankArray::new(SIZE_8_KB, 0, rom.chr_data.clone());
        chr_bank.set_bank(0, 0);

        let mut prg_banks = BankArray::new(SIZE_16_KB, 0x8000, rom.prg_data.clone());
        prg_banks.set_bank(0, 0);
        prg_banks.set_last_bank(1);

        UxROM {
            nametable_mirroring: rom.nametable_mirroring(),
            chr_bank,
            prg_banks,
        }
    }
}

impl Mapper for UxROM {
    fn read_prg(&self, address: u16) -> u8 {
        self.prg_banks.read(address)
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        self.prg_banks.read_slice(address, size)
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        if address >= 0x8000 {
            /* TODO: should be 0x7 for some variants */
            self.prg_banks.set_bank(0, (value & 0xf) as usize);
        } else {
            /* can ignore these writes */
        }
    }

    fn read_chr(&self, address: u16) -> u8 {
        self.chr_bank.read(address)
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        self.chr_bank.write(address, value);
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.nametable_mirroring.clone()
    }
}
