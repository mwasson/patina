/* the state of the cpu at a given time */
mod addressing_mode;
mod instruction;
mod status_flag;
mod program_state;

pub use addressing_mode::AddressingMode;
pub use instruction::RealizedInstruction;
pub use program_state::ProgramState;
pub use status_flag::StatusFlag;
pub use crate::cpu::instruction::from_opcode;

pub struct Operation
{
	pub instruction: RealizedInstruction,
	pub byte1: u8,
	pub byte2: u8
}

fn operation_from_memory(opcode: u8, byte1: u8, byte2: u8) -> Operation
{
	Operation {
		instruction: from_opcode(opcode),
		byte1,
		byte2
	}
}

/**
 * Converts a pair of bytes into a u16 to look up an address in memory.
 * The 6502 is little-endian, so this expects the low-order byte first.
 * addr(0xCD, 0xAB) returns 0xABCD.
 */
fn addr(lo_byte:u8, hi_byte:u8) -> u16 {
	((hi_byte as u16) << 8) + (lo_byte as u16)
}

/**
 * Zero-page address operations take a single-byte and result in an
 * address on the first page of memory, which has addresses that begin
 * with 0x00. If this is passed in 0xAB, it returns 0x00AB. In effect this
 * is just a cast, but wrapping it as a function makes the goal clearer.
 */
fn zero_page_addr(b1:u8) -> u16 {
	b1 as u16
}
