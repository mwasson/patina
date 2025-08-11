use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;
use crate::cpu;
use crate::cpu::{operation_from_memory, AddressingMode, CoreMemory, Operation, StatusFlag, INITIAL_PC_LOCATION, MEMORY_SIZE};
use crate::cpu::cpu_to_ppu_message::CpuToPpuMessage;
use crate::ppu::{PPUListener, PPURegister};
use crate::ppu::ppu_to_cpu_message::PpuToCpuMessage;
use crate::ppu::ppu_to_cpu_message::PpuToCpuMessage::PpuStatus;
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
	listener: PPUListener,
	instruction_counter: u32,
	ppu_state_receiver: Receiver<PpuToCpuMessage>,
	cycle_counter: u128,
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
	pub fn from_rom(rom: &Rom, ppu_state_receiver: Receiver<PpuToCpuMessage>, update_sender: Sender<CpuToPpuMessage>) -> Box<Self> {
		let mut memory = [0; MEMORY_SIZE];

		/* copy ROM data into memory */
		/* TODO: handling RAM, mappers, etc. */
		memory[(0x10000 - rom.prg_data.len())..0x10000].copy_from_slice(&*rom.prg_data);

		/* set program counter to value in memory at this location */

		let mut result = Self  {
			accumulator: 0x00,
			index_x: 0x00,
			index_y: 0x00,
			s_register: 0xff,
			program_counter: 0x00,
			status: (0x11) << 4,
			memory,
			listener: PPUListener::new(rom, update_sender),
			instruction_counter: 0,
			ppu_state_receiver,
			cycle_counter: 0,
		};

		result.program_counter = AddressingMode::Indirect.resolve_address_u16(&mut result, INITIAL_PC_LOCATION);

		Box::new(result)
	}

	pub fn transition(&mut self, start_time:Instant) {
		if self.handle_messages_and_check_for_nmi() {
			self.trigger_nmi();
		}

		let operation_loc = self.program_counter;
		/* TODO: what if this hits the top of program memory */
		let operation = operation_from_memory(self.read_mem(operation_loc),
											  self.read_mem(operation_loc.wrapping_add(1)),
											  self.read_mem(operation_loc.wrapping_add(2)));

		self.cycle_counter += operation.realized_instruction.cycles as u128;
		self.run_timed_from_start(self.cycle_counter, start_time, |state| {
			operation.apply(state);
		});
		self.instruction_counter += 1;
	}

	pub fn update_flag(&mut self, flag: StatusFlag, new_val: bool) {
		flag.update_bool(self, new_val);
	}

	pub fn update_zero_neg_flags(&mut self, new_val: u8) {
		self.update_flag(StatusFlag::Zero, new_val == 0);
		self.update_flag(StatusFlag::Negative, new_val & 0x80 != 0);
	}

	pub fn push(&mut self, data: u8) {
		self.write_mem(cpu::addr(self.s_register, 0x01), data);
		self.s_register = self.s_register.wrapping_sub(1);
	}

	pub fn push_memory_loc(&mut self, mem_loc: u16) {
		self.push((mem_loc >> 8) as u8);
		self.push((mem_loc & 0xff) as u8);
	}

	pub fn pop_memory_loc(&mut self) -> u16 {
		let lower = self.pop();
		let upper = self.pop();

		cpu::addr(lower, upper)
	}

	pub fn pop(&mut self) -> u8 {
		self.s_register += 1;
		let value = self.read_mem(0x0100 + self.s_register as u16);
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

	pub fn addr_from_mem(&mut self, addr_to_lookup: u16) -> u16 {
		self.addr_from_mem_separate_bytes(addr_to_lookup, addr_to_lookup+1)
	}

	pub fn addr_from_mem_separate_bytes(&mut self,
	                                    lo_byte_addr: u16,
	                                    hi_byte_addr: u16)
			-> u16 {
		cpu::addr(self.read_mem(lo_byte_addr), self.read_mem(hi_byte_addr))
	}

	fn handle_messages_and_check_for_nmi(&mut self) -> bool {
		let mut has_nmi = false;

		for message in self.ppu_state_receiver.try_recv() {
			match message {
				PpuToCpuMessage::NMI => {
					has_nmi = true;
				}
				PpuStatus(status) => {
					self.listener.ppu_status = status;
				}
			}
		}

		has_nmi
	}

	fn trigger_nmi(&mut self) {
		/* push PC onto stack */
		self.push_memory_loc(self.program_counter);
		/* push processor status register on stack */
		self.push(self.status);
		/* read NMI handler address from 0xFFFA/0xFFFB and jump to that address*/
		self.program_counter = AddressingMode::Indirect.resolve_address_u16(self, 0xfffa);
	}

	pub fn write_mem(&mut self, addr: u16, data: u8) {
		let mapped_addr = self.map_address(addr) as u16;
		let possible_ppu_register = PPURegister::from_addr(mapped_addr);
		if possible_ppu_register.is_some() {
			let register = possible_ppu_register.unwrap();
			self.listener.listen_write(&mut self.memory, &register, data);
		} else {
			self.memory[self.map_address(addr)] = data;
		}
	}

	pub fn read_mem(&mut self, addr: u16) -> u8 {
		let mapped_addr = self.map_address(addr) as u16;
		/* in some cases, the listener can modify the results */
		let possible_ppu_register = PPURegister::from_addr(mapped_addr);
		if possible_ppu_register.is_some() {
			let register = possible_ppu_register.unwrap();
			self.listener.listen_read(&register)
		} else {
			self.memory[self.map_address(addr)]
		}
	}

	fn map_address(&self, addr: u16) -> usize {
		let mapped_addr = if addr > 0x7ff && addr <= 0x1fff {
			addr & 0x7ff
		} else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
			0x2000 | (addr & 0x7)
		} else {
			addr
		};

		mapped_addr as usize
	}
}