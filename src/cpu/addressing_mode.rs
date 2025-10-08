use crate::cpu::{addr, zero_page_addr, SharedItems, CPU};
use crate::mapper::Mapper;

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
    pub fn resolve_address(self: &AddressingMode, cpu: &mut CPU, mapper: &dyn Mapper, byte1: u8, byte2: u8) -> u16 {
        let result = match self {
            AddressingMode::Implicit => panic!("Should never be explicitly referenced--remove?"),
            AddressingMode::Accumulator => panic!("Should never be explicitly referenced--remove?"),
            AddressingMode::Immediate => panic!("Immediate mode shouldn't look up in memory"),
            AddressingMode::ZeroPage => zero_page_addr(byte1),
            AddressingMode::ZeroPageX => zero_page_addr(byte1.wrapping_add(cpu.index_x)),
            AddressingMode::ZeroPageY => zero_page_addr(byte1.wrapping_add(cpu.index_y)),
            AddressingMode::Relative => {
                cpu.program_counter
                    .overflowing_add_signed(byte1 as i8 as i16)
                    .0
            }
            AddressingMode::Absolute => addr(byte1, byte2),
            AddressingMode::AbsoluteX => addr(byte1, byte2) + cpu.index_x as u16,
            AddressingMode::AbsoluteY => addr(byte1, byte2) + cpu.index_y as u16,
            AddressingMode::Indirect =>
            /* only used for JMP */
            /* this implements a bug where this mode does not
             * correctly handle crossing page boundaries
             */
            {
                cpu.addr_from_mem16(mapper, addr(byte1, byte2))
            }
            AddressingMode::IndirectX => {
                cpu.read_mem16(mapper, zero_page_addr(byte1.wrapping_add(cpu.index_x)))
            }
            AddressingMode::IndirectY => cpu.read_mem16(mapper, zero_page_addr(byte1)) + cpu.index_y as u16,
        };

        result
    }

    /* convenience method for when you have a u16 representing an entire memory address */
    pub fn resolve_address_u16(&self, cpu: &mut CPU, mapper: &dyn Mapper, addr: u16) -> u16 {
        self.resolve_address(cpu, mapper, (addr & 0xff) as u8, (addr >> 8) as u8)
    }

    pub fn deref(self: &AddressingMode, cpu: &mut CPU, shared_items: &mut SharedItems, byte1: u8, byte2: u8) -> u8 {
        match self {
            AddressingMode::Immediate => byte1,
            AddressingMode::Accumulator => cpu.accumulator,
            _ => {
                let address = self.resolve_address(cpu, shared_items.mapper, byte1, byte2);
                cpu.read_mem(shared_items, address)
            }
        }
    }

    pub fn write(self: &AddressingMode, cpu: &mut CPU, shared_items: &mut SharedItems, byte1: u8, byte2: u8, new_val: u8) {
        match self {
            AddressingMode::Accumulator => cpu.accumulator = new_val,
            _ => {
                let resolved_addr = self.resolve_address(cpu, shared_items.mapper, byte1, byte2);
                cpu.write_mem(shared_items, resolved_addr, new_val)
            }
        }
    }
}
