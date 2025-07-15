use crate::cpu::{addr, zero_page_addr, ProgramState};

#[derive(Debug)]
pub enum AddressingMode
{
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

impl AddressingMode
{
	/* behavior based on: https://www.nesdev.org/obelisk-6502-guide/addressing.html */
	pub fn resolve_address(self: &AddressingMode, state: &ProgramState, byte1:u8, byte2:u8) -> u16 {
		match self  {
			AddressingMode::Implicit =>
				panic!("Should never be explicitly referenced--remove?"),
			AddressingMode::Accumulator =>
				panic!("Should never be explicitly referenced--remove?"),
			AddressingMode::Immediate =>
				panic!("Immediate mode shouldn't look up in memory"),
			AddressingMode::ZeroPage =>
				zero_page_addr(byte1),
			AddressingMode::ZeroPageX =>
				zero_page_addr(byte1 + state.index_x),
			AddressingMode::ZeroPageY =>
				zero_page_addr(byte1 + state.index_y),
			AddressingMode::Relative =>
				state.program_counter
				     .overflowing_add_signed(byte1 as i8 as i16).0,
			AddressingMode::Absolute =>
				addr(byte1, byte2),
			AddressingMode::AbsoluteX =>
				addr(byte1 + state.index_x, byte2),
			AddressingMode::AbsoluteY =>
				addr(byte1 + state.index_y, byte2),
			AddressingMode::Indirect => /* only used for JMP */
				/* this implements a bug where this mode does not
				 * correctly handle crossing page boundaries
				 */
                state.addr_from_mem_separate_bytes(addr(byte1, byte2),
                                                   addr(byte1+1, byte2)),
            AddressingMode::IndirectX =>
				state.addr_from_mem(zero_page_addr(byte1 + state.index_x)),
			AddressingMode::IndirectY =>
				state.addr_from_mem(zero_page_addr(byte1 + state.index_y)),
		}
	}

	pub fn deref(self: &AddressingMode, state: &ProgramState, byte1:u8, byte2:u8) -> u8 {
		match self {
			AddressingMode::Immediate => byte1,
			_ => {
				let address = self.resolve_address(state, byte1, byte2);
				state.read_mem(address)
			}
		}
	}

	pub fn write(self: &AddressingMode, state: &mut ProgramState, byte1: u8, byte2: u8, new_val: u8) {
		state.write_mem(self.resolve_address(state, byte1, byte2), new_val);
	}
}