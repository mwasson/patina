use crate::mapper::Mapper;
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

const SHIFT_REGISTER_INITIAL_VAL : u8 = 1 << 4;

/* TODO does not necessarily correctly handle consecutive cycle writes; this might become
 * an issue if we reach cycle accuracy
 */

/* TODO only implements default MMC1 behavior, does not handle SEROM, SHROM, SH1ROM, etc. */

enum PrgRomBankMode {
    Mode32kb, /* switch 32kb banks, ignoring low bit of bank number */
    Mode16KbFixLower, /* 16kb banks, 0x8000 fixed to first bank, switch 0xc000 */
    Mode16KbFixUpper, /* 16kb banks, switch 0x8000, 0xc000 fixed to last bank */
}

pub struct MMC1 {
    shift_register: u8,
    prg_ram: Box<[u8; 1 << 15]>, /* optional RAM; TODO size can only be determined in NES 2.0 ROMs*/
    prg_rom: Box<Vec<u8>>,
    chr_ram: Box<Vec<u8>>, /* might be ROM */
    ram_bank_index: u8,
    chr_bank_0: usize,
    chr_bank_1: usize,
    chr_bank_mode: bool, /* true == switch two 4kb banks; false == switch single 8kb bank */
    prg_bank_mode: PrgRomBankMode,
    prg_bank_index: usize,
    nametable_mirroring: NametableMirroring,
}

impl MMC1 {
    pub fn new(rom: &Rom) -> MMC1 {
        /* TODO AWFUL--but length of chr data in rom doesn't determine CHR-RAM space */
        let mut chr_rom = Vec::with_capacity(1 << 16);
        for i in 0..(1<<16) {
            chr_rom.push(0);
        }
        for i in 0 ..rom.chr_data.len() {
            chr_rom[i] = rom.chr_data[i];
        }

        let prg_rom = Box::new(rom.prg_data.clone());

        MMC1 {
            shift_register: SHIFT_REGISTER_INITIAL_VAL,
            prg_ram: Box::new([0; 1 << 15]),
            prg_rom,
            chr_ram: Box::new(chr_rom), /* NB: might actually be CHR-RAM */
            ram_bank_index: 0,
            chr_bank_0: 0,
            chr_bank_1: 1,
            chr_bank_mode: false,
            prg_bank_mode: PrgRomBankMode::Mode16KbFixUpper, /* empirically determined default mode */
            prg_bank_index: 0,
            nametable_mirroring: NametableMirroring::Horizontal,
        }
    }

    fn listen_for_state_change(&mut self, address: u16, value: u8) {
        /* a write with bit 7 set resets the shift register */
        if value & 0x80 != 0 {
            self.shift_register = SHIFT_REGISTER_INITIAL_VAL;
        } else {
            /* the shift register is full when the initial 1 has reached the end */
            let shift_register_full = self.shift_register & 1 != 0;

            /* shift it over, putting the lowest order bit from the write on it */
            self.shift_register >>= 1;
            self.shift_register |= (value & 1) << 4;

            /* if we're full, write to the appropriate register, based on where the fifth write
             * occurred, then empty the shift register
             */
            if (shift_register_full) {
                /* control register */
                if address < 0xa000 {
                    self.nametable_mirroring = match self.shift_register & 3 {
                        0 => NametableMirroring::Single, /* TODO 'lower bank' */
                        1 => NametableMirroring::Single, /* TODO 'upper bank' */
                        2 => NametableMirroring::Horizontal,
                        _ => NametableMirroring::Vertical,
                    };
                    /* bits 2 and 3: PRG-ROM bank mode */
                    self.prg_bank_mode = match (self.shift_register >> 2) & 3 {
                        2 => PrgRomBankMode::Mode16KbFixLower,
                        3 => PrgRomBankMode::Mode16KbFixUpper,
                        _ => PrgRomBankMode::Mode32kb, /* 0 or 1 */
                    };
                    /* bit 4: CHR-ROM bank mode: 1 == switch 8kb, 0 == switch 4kb */
                    self.chr_bank_mode = value & 0x10 != 0;
                /* CHR bank 0 */
                } else if address < 0xc000 {
                    self.write_chr_bank_data(false, self.shift_register);
                /* CHR bank 1 */
                } else if address < 0xe000 {
                    self.write_chr_bank_data(true, self.shift_register);
                /* PRG bank */
                } else {
                    /* bits 0-3: select 16kb PRG-ROM bank, first bit ignored in 32kb mode */
                    self.prg_bank_index = (self.shift_register & 0xf) as usize;
                    /* TODO: bit 4 disables the PRG-RAM chip. In practice, what does that mean? */
                }
                self.shift_register = SHIFT_REGISTER_INITIAL_VAL;
            }
        }
    }

    fn write_chr_bank_data(&mut self, is_bank_1: bool, data: u8) {
        if is_bank_1 {
            self.chr_bank_1 = data as usize
        } else {
            self.chr_bank_0 = data as usize
        }
    }

    fn prg_rom_index(&self, address: u16) -> usize {
        match self.prg_bank_mode {
            PrgRomBankMode::Mode32kb => {
                (self.prg_bank_index >> 1) * 0x8000 + address as usize - 0x8000
            },
            PrgRomBankMode::Mode16KbFixLower => {
                if address < 0xc000 {
                    /* read from first bank */
                    address as usize - 0x8000
                } else {
                    self.prg_bank_index * 0x4000 + address as usize - 0xc000
                }
            },
            PrgRomBankMode::Mode16KbFixUpper => {
                if address < 0xc000 {
                    self.prg_bank_index * 0x4000 + address as usize - 0x8000
                } else {
                    /* read from last bank */
                    self.prg_rom.len() - 0x4000 + address as usize - 0xc000
                }
            }
        }
    }

    fn prg_ram_index(&self, address: u16) -> usize {
        (address as usize - 0x6000) + (self.ram_bank_index as usize * 0x2000)
    }

    fn chr_index(&self, address: u16) -> usize {
        let bank_size: usize = if self.chr_bank_mode { 0x1000 } else { 0x2000 };
        let is_bank_1 = address & 0x1000 != 0;
        /* 4kb mode */
        if self.chr_bank_mode {
            let bank_num = if is_bank_1 { self.chr_bank_1 } else { self.chr_bank_0 };
            let bank_offset = if is_bank_1 { 0x1000 } else { 0 };
            bank_size * bank_num + (address - bank_offset) as usize
        /* 8kb mode */
        } else {
            bank_size * (self.chr_bank_0 >> 1) + address as usize
        }
    }
}

impl Mapper for MMC1 {
    fn read_prg(&self, address: u16) -> u8 {
        if address < 0x8000 {
            self.prg_ram[self.prg_ram_index(address)]
        } else {
            self.prg_rom[self.prg_rom_index(address)]
        }
    }

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8] {
        if address < 0x8000 {
            let index = self.prg_ram_index(address);
            &self.prg_ram[index..index+size]
        } else {
            let index = self.prg_rom_index(address);
            &self.prg_rom[index..index+size]
        }
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        let usize_addr = address as usize;

        /* below 0x8000, it's writing to PRG-RAM, which we assume exists TODO update for NES 2.0 */
        if usize_addr < 0x8000 {
            self.prg_ram[(usize_addr - 0x6000) + (self.ram_bank_index as usize * 0x2000)] = value;
        /* otherwise, we don't actually write: these addresses are only for changing
         * MMC1 internal state
         */
        } else {
            self.listen_for_state_change(address, value);
        }
    }

    fn read_chr(&self, address: u16) -> u8 {
        self.chr_ram[self.chr_index(address)]
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        let index = self.chr_index(address);
        self.chr_ram[index] = value;
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.nametable_mirroring.clone()
    }
}