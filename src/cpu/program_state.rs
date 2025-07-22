use std::sync::{Arc, Mutex};
use crate::cpu;
use crate::cpu::{AddressingMode, StatusFlag, INITIAL_PC_LOCATION, MEMORY_SIZE, RAM_MEMORY_START};
use crate::cpu::core_memory::CoreMemory;
use crate::ppu::PPUState;
use crate::processor::Processor;
use crate::rom::Rom;

pub struct ProgramState
{
	pub accumulator: u8,
	pub index_x: u8,
	pub index_y: u8,
	pub s_register: u8,
	pub program_counter: u16,
	pub status: u8,
	memory: CoreMemory,
}

impl Processor for ProgramState
{
	fn clock_speed(&self) -> u64 {
		1790000 /* 1.79 MHz */
	}
}

impl ProgramState
{
	/* TODO comment */
	pub fn from_rom(rom: &Rom) -> Self {
		let mut memory = [0; MEMORY_SIZE];

		/* copy ROM data into memory */
		/* TODO: handling RAM, mappers, etc. */
		memory[RAM_MEMORY_START..(RAM_MEMORY_START+rom.prg_data.len())].copy_from_slice(&*rom.prg_data);
		memory[(RAM_MEMORY_START-rom.chr_data.len())..RAM_MEMORY_START].copy_from_slice(&*rom.chr_data);

		/* set program counter to value in memory at this location */


		let mut result = Self  {
			accumulator: 0x00,
			index_x: 0x00,
			index_y: 0x00,
			s_register: 0xff,
			program_counter: 0x00,
			status: (0x11) << 4,
			memory: CoreMemory::new(memory)
		};

		result.program_counter = AddressingMode::Indirect.resolve_address_u16(&result, INITIAL_PC_LOCATION);

		result
	}

	pub fn update_flag(&mut self, flag: StatusFlag, new_val: bool) {
		flag.update_bool(self, new_val);
	}

	pub fn update_zero_neg_flags(&mut self, new_val: u8) {
		self.update_flag(StatusFlag::Zero, new_val == 0);
		self.update_flag(StatusFlag::Negative, (new_val as i8) < 0);
	}

	pub fn push(&mut self, data: u8) {
		self.memory.write(cpu::addr(self.s_register, 0x10), data);
		self.s_register = self.s_register.wrapping_sub(1);
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
		let value = self.memory.read(0x10 + self.s_register as u16);
		self.s_register += 1;
		value
	}

	pub fn irq(&mut self) {
		self.irq_with_offset(0);
	}

	pub fn irq_with_offset(&mut self, offset: u8) {
		self.push_memory_loc(self.program_counter.wrapping_add(offset as u16));
		self.push(self.status);
		self.update_flag(StatusFlag::InterruptDisable, false);
		/* TODO: jump to IRQ handler */
	}

	pub fn read_mem(&self, addr: u16) -> u8 {
		self.memory.read(addr)
	}

	pub fn write_mem(&mut self, addr: u16, data: u8) {
		self.memory.write(addr, data);
	}

	pub fn addr_from_mem(&self, addr_to_lookup: u16) -> u16 {
		self.addr_from_mem_separate_bytes(addr_to_lookup, addr_to_lookup+1)
	}

	pub fn addr_from_mem_separate_bytes(&self,
	                                    lo_byte_addr: u16,
	                                    hi_byte_addr: u16)
			-> u16 {
		cpu::addr(self.read_mem(lo_byte_addr), self.read_mem(hi_byte_addr))
	}
	
	pub fn clone_memory(&self) -> CoreMemory {
		self.memory.clone()
	}
}