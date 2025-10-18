use crate::cpu::{addr, zero_page_addr, CPU};

use AddressingMode::*;

#[derive(Debug)]
pub enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

impl AddressingMode {
    /* behavior based on: https://www.nesdev.org/obelisk-6502-guide/addressing.html */
    pub fn resolve_address(self: &AddressingMode, cpu: &mut CPU, byte1: u8, byte2: u8) -> u16 {
        let result = match self {
            Implicit => panic!("Should never be explicitly referenced--remove?"),
            Accumulator => panic!("Should never be explicitly referenced--remove?"),
            Immediate => panic!("Immediate mode shouldn't look up in memory"),
            ZeroPage => zero_page_addr(byte1),
            ZeroPageX => zero_page_addr(byte1.wrapping_add(cpu.index_x)),
            ZeroPageY => zero_page_addr(byte1.wrapping_add(cpu.index_y)),
            Relative => {
                cpu.program_counter
                    .overflowing_add_signed(byte1 as i8 as i16)
                    .0
            }
            Absolute => addr(byte1, byte2),
            AbsoluteX => addr(byte1, byte2) + cpu.index_x as u16,
            AbsoluteY => addr(byte1, byte2) + cpu.index_y as u16,
            Indirect =>
            /* only used for JMP */
            /* this implements a bug where this mode does not
             * correctly handle crossing page boundaries
             */
            {
                cpu.addr_from_mem16(addr(byte1, byte2))
            }
            IndirectX => cpu.read_mem16(zero_page_addr(byte1.wrapping_add(cpu.index_x))),
            IndirectY => cpu.read_mem16(zero_page_addr(byte1)) + cpu.index_y as u16,
        };

        result
    }

    pub fn get_bytes(&self) -> u8 {
        match self {
            Implicit => 1,
            Accumulator => 1,
            Immediate => 2,
            ZeroPage => 2,
            ZeroPageX => 2,
            ZeroPageY => 2,
            Relative => 2,
            Absolute => 3,
            AbsoluteX => 3,
            AbsoluteY => 3,
            Indirect => 3,
            IndirectX => 2,
            IndirectY => 2,
        }
    }

    /* convenience method for when you have a u16 representing an entire memory address */
    pub fn resolve_address_u16(&self, cpu: &mut CPU, addr: u16) -> u16 {
        self.resolve_address(cpu, (addr & 0xff) as u8, (addr >> 8) as u8)
    }

    pub fn deref(self: &AddressingMode, cpu: &mut CPU, byte1: u8, byte2: u8) -> u8 {
        match self {
            Immediate => byte1,
            Accumulator => cpu.accumulator,
            _ => {
                let address = self.resolve_address(cpu, byte1, byte2);
                cpu.read_mem(address)
            }
        }
    }

    pub fn write(self: &AddressingMode, cpu: &mut CPU, byte1: u8, byte2: u8, new_val: u8) {
        match self {
            AddressingMode::Accumulator => cpu.accumulator = new_val,
            _ => {
                let resolved_addr = self.resolve_address(cpu, byte1, byte2);
                cpu.write_mem(resolved_addr, new_val)
            }
        }
    }
}
