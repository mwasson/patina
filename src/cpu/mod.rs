/* the state of the cpu at a given time */
mod addressing_mode;
mod instruction;
mod status_flag;
mod program_state;
mod memory_mirror;

pub use addressing_mode::AddressingMode;
pub use instruction::RealizedInstruction;
pub use program_state::ProgramState;
pub use status_flag::StatusFlag;
pub use crate::cpu::instruction::from_opcode;
use crate::rom::Rom;

const MEMORY_SIZE: usize = 1<<15;

const INITIAL_PC_LOCATION: u16 = 0xfffc;

#[derive(Debug)]
pub struct Operation
{
	pub realized_instruction: RealizedInstruction, /* TODO should this be a reference? */
	pub byte1: u8,
	pub byte2: u8
}

impl Operation
{
	fn apply(&self, state: &mut ProgramState) {
		self.realized_instruction.apply(state, self.byte1, self.byte2);
	}
}

fn operation_from_memory(opcode: u8, byte1: u8, byte2: u8) -> Operation
{
	Operation {
		realized_instruction: from_opcode(opcode),
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

fn transition(rom:&Rom, state: &mut ProgramState) {
	let operation_loc = state.program_counter as usize;
	/* TODO: what if this hits the top of program memory */
	let operation = operation_from_memory(rom.prg_data[operation_loc],
										  rom.prg_data[operation_loc+1],
										  rom.prg_data[operation_loc+2]);
	println!("Running operation: {operation:?}");
	operation.apply(state)
}

/* TODO: very basic test of CPU */
pub fn operate(rom:&Rom, state: &mut ProgramState) {
	while((state.program_counter as usize) < rom.prg_data.len()) {
		println!("Taking a step at program counter {0}", state.program_counter);
		transition(rom, state);
	}
	println!("Program counter exceeded size of program memory");
}
