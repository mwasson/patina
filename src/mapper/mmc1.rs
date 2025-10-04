use crate::mapper::bank_array::BankArray;
use crate::mapper::{Mapper, SIZE_16_KB, SIZE_32_KB, SIZE_4_KB, SIZE_8_KB};
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

const SHIFT_REGISTER_INITIAL_VAL: u8 = 1 << 4;

/* TODO does not necessarily correctly handle consecutive cycle writes; this might become
 * an issue if we reach cycle accuracy
 */

/* TODO only implements default MMC1 behavior, does not handle SEROM, SHROM, SH1ROM, etc. */

#[derive(Debug)]
enum PrgRomBankMode {
    Mode32kb,         /* switch 32kb banks, ignoring low bit of bank number */
    Mode16KbFixLower, /* 16kb banks, 0x8000 fixed to first bank, switch 0xc000 */
    Mode16KbFixUpper, /* 16kb banks, switch 0x8000, 0xc000 fixed to last bank */
}

pub struct MMC1 {
    shift_register: u8,
    prg_ram: Box<[u8; SIZE_32_KB as usize]>, /* optional RAM; TODO size can only be determined in NES 2.0 ROMs*/
    prg_banks: BankArray,
    chr_banks: BankArray,
    chr_bank_0: u8,
    chr_bank_1: u8,
    chr_bank_mode: bool, /* true == switch two 4kb banks; false == switch single 8kb bank */
    prg_bank_mode: PrgRomBankMode,
    prg_bank_index: usize,
    nametable_mirroring: NametableMirroring,
}

impl MMC1 {
    pub fn new(rom: &Rom) -> MMC1 {
        let prg_banks = BankArray::new(SIZE_16_KB, 0x8000, rom.prg_data.clone());
        let chr_banks = BankArray::new(SIZE_8_KB, 0, rom.chr_data.clone());

        let mut result = MMC1 {
            shift_register: SHIFT_REGISTER_INITIAL_VAL,
            prg_ram: Box::new([0; 1 << 15]),
            prg_banks,
            chr_banks,
            chr_bank_0: 0,
            chr_bank_1: 1,
            chr_bank_mode: false,
            prg_bank_mode: PrgRomBankMode::Mode16KbFixUpper, /* empirically determined default mode */
            prg_bank_index: 0,
            nametable_mirroring: NametableMirroring::Horizontal,
        };

        result.update_prg_banks();
        result.update_chr_banks();

        result
    }

    fn listen_for_state_change(&mut self, address: u16, value: u8) {
        /* a write with bit 7 set resets the mapper */
        if value & 0x80 != 0 {
            self.shift_register = SHIFT_REGISTER_INITIAL_VAL;
            self.prg_bank_mode = PrgRomBankMode::Mode16KbFixUpper;
        } else {
            /* the shift register is full when the initial 1 has reached the end */
            let shift_register_full = self.shift_register & 1 != 0;

            /* shift it over, putting the lowest order bit from the write on it */
            self.shift_register >>= 1;
            self.shift_register |= (value & 1) << 4;

            /* if we're full, write to the appropriate register, based on where the fifth write
             * occurred, then empty the shift register
             */
            if shift_register_full {
                /* control register */
                if address < 0xa000 {
                    self.nametable_mirroring = match self.shift_register & 3 {
                        0 => NametableMirroring::SingleNametable0,
                        1 => NametableMirroring::SingleNametable1,
                        2 => NametableMirroring::Horizontal,
                        _ => NametableMirroring::Vertical,
                    };
                    /* bits 2 and 3: PRG-ROM bank mode */
                    self.prg_bank_mode = match (self.shift_register >> 2) & 3 {
                        2 => PrgRomBankMode::Mode16KbFixLower,
                        3 => PrgRomBankMode::Mode16KbFixUpper,
                        _ => PrgRomBankMode::Mode32kb, /* 0 or 1 */
                    };
                    self.update_prg_banks();
                    /* bit 4: CHR-ROM bank mode: 1 == switch 8kb, 0 == switch 4kb */
                    self.chr_bank_mode = self.shift_register & 0x10 != 0;
                    self.update_chr_banks();
                /* CHR bank 0 */
                } else if address < 0xc000 {
                    self.chr_bank_0 = self.shift_register;
                    self.update_chr_banks();
                /* CHR bank 1 */
                } else if address < 0xe000 {
                    self.chr_bank_1 = self.shift_register;
                    self.update_chr_banks();
                /* PRG bank */
                } else {
                    self.prg_bank_index = (self.shift_register & 0xf) as usize;
                    self.update_prg_banks();
                    /* TODO: bit 4 disables the PRG-RAM chip. In practice, what does that mean? */
                }
                self.shift_register = SHIFT_REGISTER_INITIAL_VAL;
            }
        }
    }

    fn update_prg_banks(&mut self) {
        match self.prg_bank_mode {
            PrgRomBankMode::Mode32kb => {
                self.prg_banks.change_bank_size(SIZE_32_KB);
                /* first bit ignored in 32kb mode */
                self.prg_banks.set_bank(0, self.prg_bank_index >> 1);
            }
            PrgRomBankMode::Mode16KbFixLower => {
                self.prg_banks.change_bank_size(SIZE_16_KB);
                self.prg_banks.set_bank(0, 0);
                self.prg_banks.set_bank(1, self.prg_bank_index);
            }
            PrgRomBankMode::Mode16KbFixUpper => {
                self.prg_banks.change_bank_size(SIZE_16_KB);
                self.prg_banks.set_bank(0, self.prg_bank_index);
                self.prg_banks.set_last_bank(1);
            }
        }
    }

    fn update_chr_banks(&mut self) {
        if self.chr_bank_mode {
            self.chr_banks.change_bank_size(SIZE_4_KB);
            self.chr_banks.set_bank(0, self.chr_bank_0 as usize);
            self.chr_banks.set_bank(1, self.chr_bank_1 as usize);
        } else {
            self.chr_banks.change_bank_size(SIZE_8_KB);
            self.chr_banks.set_bank(0, (self.chr_bank_0 >> 1) as usize);
        }
    }

    fn prg_ram_index(&self, address: u16) -> usize {
        address as usize - 0x6000
    }
}

impl Mapper for MMC1 {
    fn read_prg(&self, address: u16) -> u8 {
        if address < 0x8000 {
            self.prg_ram[self.prg_ram_index(address)]
        } else {
            self.prg_banks.read(address)
        }
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        if address < 0x8000 {
            let index = self.prg_ram_index(address);
            &self.prg_ram[index..index + size]
        } else {
            self.prg_banks.read_slice(address, size)
        }
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        /* below 0x8000, it's writing to PRG-RAM, which we assume exists TODO update for NES 2.0 */
        if address < 0x8000 {
            self.prg_ram[self.prg_ram_index(address)] = value;
        /* otherwise, writing to an MMC1 register */
        } else {
            self.listen_for_state_change(address, value);
        }
    }

    fn read_chr(&self, address: u16) -> u8 {
        self.chr_banks.read(address)
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        self.chr_banks.write(address, value);
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.nametable_mirroring.clone()
    }
}
