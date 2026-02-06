use crate::mapper::bank_array::BankArray;
use crate::mapper::{Mapper, SIZE_32_KB, SIZE_8_KB};
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

pub struct AxROM {
    prg_banks: BankArray,
    chr_bank: BankArray,
    nametable_mirroring: NametableMirroring,
}

impl AxROM {
    pub fn new(rom: &Rom) -> Self {
        let chr_bank = BankArray::new_ram(0, SIZE_8_KB, SIZE_8_KB);

        let prg_banks = BankArray::new(0x8000, SIZE_32_KB, rom.prg_data.clone());

        AxROM {
            nametable_mirroring: rom.nametable_mirroring(),
            chr_bank,
            prg_banks,
        }
    }
}

impl Mapper for AxROM {
    fn read_prg(&self, address: u16) -> u8 {
        self.prg_banks.read(address)
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        self.prg_banks.read_slice(address, size)
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

    fn write_prg_ram(&mut self, address: u16, data: u8) {
        todo!()
    }

    fn write_prg_rom(&mut self, address: u16, data: u8) {
        self.prg_banks.set_bank(0,data & 0x7);
        self.nametable_mirroring = if data & 0x10 != 0 {
            NametableMirroring::SingleNametable1
        } else {
            NametableMirroring::SingleNametable0
        };
    }
}
