/* the state of the cpu at a given time */
mod addressing_mode;
mod instruction;
mod status_flag;

pub use instruction::Instruction;
pub use instruction::RealizedInstruction;
pub use status_flag::StatusFlag;
pub use addressing_mode::AddressingMode;
pub use crate::cpu::instruction::from_opcode;

pub struct ProgramState
{
	accumulator: u8,
	index_x: u8,
	index_y: u8,
	s_register: u8,
	program_counter: u16,
	status: u8,
	memory: [u8; 1<<15]
}

impl ProgramState
{
	pub fn new() -> Self {
		Self {
			accumulator: 0x00,
			index_x: 0x00,
			index_y: 0x00,
			s_register: 0xff,
			program_counter: 0x0000,
			status: (0x11) << 4,
			memory: [0; 1<<15]
		}
	}

	pub fn update_flag(&mut self, flag: StatusFlag, new_val: bool) {
		flag.update_bool(self, new_val);
	}

	pub fn update_zero_neg_flags(&mut self, new_val: u8) {
		self.update_flag(StatusFlag::Zero, new_val == 0);
		self.update_flag(StatusFlag::Negative, new_val.leading_ones() > 0);
	}
		
	pub fn push(&mut self, data: u8) {
		self.memory[addr(self.s_register, 0x10) as usize] = data;
		self.s_register -= 1;
	}

	pub fn push_memory_loc(&mut self, mem_loc: u16) {
		self.push(mem_loc as u8);
		self.push((mem_loc >> 8) as u8);
	}

	pub fn pop_memory_loc(&mut self) -> u16 {
		let upper = self.pop();
		let lower = self.pop();

		addr(lower, upper)
	}

	pub fn pop(&mut self) -> u8 {
		let value = self.memory[(0x10 + self.s_register) as usize];
		self.s_register += 1;
		value
	}	

	pub fn irq(&mut self) {
		self.irq_with_offset(0);
	}

	pub fn irq_with_offset(&mut self, offset: u8) {
		self.push_memory_loc(self.program_counter + offset as u16);
		self.push(self.status);
		self.update_flag(StatusFlag::InterruptDisable, false);
		/* TODO: jump to IRQ handler */
	}

	pub fn mem_lookup(&mut self, addr: u16) -> u8 {
		self.memory[addr as usize]
	}

	pub fn addr_from_mem(&mut self, addr_to_lookup: u16) -> u16 {
		self.addr_from_mem_separate_bytes(addr_to_lookup, addr_to_lookup+1)
	}
				
	pub fn addr_from_mem_separate_bytes(&mut self,
	                                    lo_byte_addr: u16,
	                                    hi_byte_addr: u16)
			-> u16 {	
		addr(self.mem_lookup(lo_byte_addr), self.mem_lookup(hi_byte_addr))
	}
}

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
