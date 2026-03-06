use crate::cpu::CPU;
use crate::mapper::{Mapper, RamBank, SIZE_1_KB, SIZE_8_KB};
use crate::mapper::bank_array::BankArray;
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

pub struct MMC3 {
    prg_ram: RamBank, /* TODO: optional */
    prg_banks: BankArray,
    chr_banks: BankArray,
    mirroring: NametableMirroring,
    irq_counter: u8,
    irq_reload: u8,
    irq_enabled: bool,
    prg_bank_mode: bool,
    chr_a12_inversion: bool,
    prg_ram_write_protect: bool,
    prg_ram_enabled: bool,
    selected_bank: Option<NamedBank>
}

struct NamedBank {
    is_chr: bool,
    is_double: bool,
    regular_mapping: u8,
    inverted_mapping: u8,
}

impl NamedBank {
    const R0 : NamedBank  = NamedBank::new(true,true,0,4);
    const R1 : NamedBank  = NamedBank::new(true,true,2,6);
    const R2 : NamedBank  = NamedBank::new(true,false,4,0);
    const R3 : NamedBank  = NamedBank::new(true,false,5,1);
    const R4 : NamedBank  = NamedBank::new(true,false,6,2);
    const R5 : NamedBank  = NamedBank::new(true,false,7,3);
    const R6 : NamedBank  = NamedBank::new(false,false,0,2);
    const R7 : NamedBank  = NamedBank::new(false,false,1,1);

    const fn new(is_chr: bool, is_double: bool, regular_mapping: u8, inverted_mapping: u8) -> NamedBank {
        Self {
            is_chr, is_double, regular_mapping, inverted_mapping
        }
    }

    fn bank_index(&self, invert: bool) -> u8 {
        if invert { self.inverted_mapping } else { self.regular_mapping }
    }
}

impl MMC3 {
    pub fn new(rom: &Rom) -> Self {
        let mut prg_banks = BankArray::new(0x8000, SIZE_8_KB, rom.prg_data.clone(), 4);
        prg_banks.set_bank_from_end(2,-2);
        prg_banks.set_bank_from_end(3, -1);

        /* TODO */
        let chr_banks = BankArray::new(0, SIZE_1_KB, rom.chr_data.clone(), 8);

        MMC3 {
            prg_ram: Box::new([0; 1 << 15]),
            prg_banks,
            chr_banks,
            mirroring: initial_nametable_mirroring(&rom),
            irq_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
            prg_bank_mode: false,
            chr_a12_inversion: false,
            selected_bank: None,
            prg_ram_write_protect: false,
            prg_ram_enabled: true,
        }
    }

    fn prg_ram_index(&self, address: u16) -> usize {
        address as usize - 0x6000
    }

    fn listen_for_state_change(&mut self, address: u16, value: u8) {
        let is_even = address & 1 == 0;
        if(address >= 0x8000 && address < 0xa000) {
            /* 0x8000-0x9fff even: bank select */
            if is_even {
                self.bank_select(value);
            /* 0x8000-0x9fff odd: bank data */
            } else {
                self.bank_data(value);
            }
        } else if address < 0xc000 {
            /* 0xa000-0xbfff even: nametable arrangement */
            if is_even {
                self.mirroring = if value & 1 != 0 { NametableMirroring::Vertical } else { NametableMirroring::Horizontal };
            /* 0xa000-0xbfff odd: prg ram protect */
            } else {
                self.prg_ram_enabled = value & 0x40 != 0;
                self.prg_ram_write_protect = value & 0x80 != 0;
            }
        } else if address < 0xe000 {
            /* 0xc000-0xdfff even: irq latch */
            if is_even {
                self.irq_reload = value;
            /* 0xc000-0xdfff odd: irq reload */
            } else {
               self.irq_counter = 0;
            }
        } else {
            /* 0xe000-0xffff even: irq disable */
            /* 0xe000-0xffff odd: irq enable */
            self.irq_enabled = !is_even;
        }
    }

    fn bank_select(&mut self, value: u8) {
        let chr_a12_inversion = value & 0x80 != 0;
        if chr_a12_inversion != self.chr_a12_inversion {
            self.chr_a12_inversion = chr_a12_inversion;
        }

        let prg_bank_mode = value & 0x40 != 0;
        if prg_bank_mode != self.prg_bank_mode {
            self.prg_bank_mode = prg_bank_mode;
        }

        /* TODO: if we switch between modes, does everything work correctly? */

        self.selected_bank = Some(match(value & 0x7) {
            0b000 => NamedBank::R0,
            0b001 => NamedBank::R1,
            0b010 => NamedBank::R2,
            0b011 => NamedBank::R3,
            0b100 => NamedBank::R4,
            0b101 => NamedBank::R5,
            0b110 => NamedBank::R6,
            0b111 => NamedBank::R7,
            _ => unreachable!()
        });
    }
    fn bank_data(&mut self, value: u8) {
        if let Some(named_bank) = &self.selected_bank {
            if named_bank.is_chr {
                if named_bank.is_double {
                    let even_value = value & !1;
                    self.chr_banks.set_bank(named_bank.bank_index(self.chr_a12_inversion), even_value);
                    self.chr_banks.set_bank(named_bank.bank_index(self.chr_a12_inversion)+1, even_value+1);
                } else {
                    self.chr_banks.set_bank(named_bank.bank_index(self.chr_a12_inversion), value);
                }
            } else {
                self.prg_banks.set_bank(named_bank.bank_index(self.prg_bank_mode), value);
            }
        }
    }
}

impl Mapper for MMC3 {
    fn read_prg(&self, address: u16) -> u8 {
        if address < 0x6000 {
            return 0;
        }

        /* TODO later: seriously clean this up, prg_ram should be split out better */
        if address < 0x8000 {
            if self.prg_ram_enabled { self.prg_ram[self.prg_ram_index(address)] } else { 0 /* TODO open bus */ }
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

    fn read_chr(&self, address: u16) -> u8 {
        self.chr_banks.read(address)
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        self.chr_banks.write(address, value);
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.mirroring.clone()
    }

    fn get_save_data(&self) -> Option<Vec<u8>> {
        Some(Vec::from(*self.prg_ram))
    }

    fn set_save_data(&mut self, data: &Vec<u8>) {
        self.prg_ram[0..data.len()].copy_from_slice(data);
    }

    fn write_prg_ram(&mut self, address: u16, data: u8) {
        if !self.prg_ram_write_protect {
            self.prg_ram[self.prg_ram_index(address)] = data;
        }
    }

    fn write_prg_rom(&mut self, address: u16, data: u8) {
        self.listen_for_state_change(address, data);
    }

    fn listen_ppu_a12(&mut self, cpu: &mut CPU) {
        if self.irq_counter == 0 {
            self.irq_counter = self.irq_reload;
        } else {
            self.irq_counter -= 1;

            if self.irq_counter == 0 && self.irq_enabled {
                cpu.set_irq(true);
            }
        }
    }
}

fn initial_nametable_mirroring(rom: &Rom) -> NametableMirroring {
    if rom.byte_6_flags & (1 << 3) != 0 { NametableMirroring::FourScreen } else { NametableMirroring::Horizontal }
}