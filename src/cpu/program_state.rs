use crate::cpu;
use crate::cpu::StatusFlag;

pub struct ProgramState
{
	pub accumulator: u8,
	pub index_x: u8,
	pub index_y: u8,
	pub s_register: u8,
	pub program_counter: u16,
	pub status: u8,
	pub memory: [u8; 1<<15]
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
		self.memory[cpu::addr(self.s_register, 0x10) as usize] = data;
		self.s_register -= 1;
	}

	pub fn push_memory_loc(&mut self, mem_loc: u16) {
		self.push(mem_loc as u8);
		self.push((mem_loc >> 8) as u8);
	}

	pub fn pop_memory_loc(&mut self) -> u16 {
		let upper = self.pop();
		let lower = self.pop();

		cpu::addr(lower, upper)
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
		cpu::addr(self.mem_lookup(lo_byte_addr), self.mem_lookup(hi_byte_addr))
	}
}