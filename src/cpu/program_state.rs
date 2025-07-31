use std::time;
use std::time::Instant;
use crate::cpu;
use crate::cpu::{from_opcode, AddressingMode, Operation, StatusFlag, INITIAL_PC_LOCATION, MEMORY_SIZE};
use crate::cpu::core_memory::CoreMemory;
use crate::cpu::instruction::Instruction;
use crate::ppu::{PPUListener};
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
	instruction_counter: u32,
	current_instruction: Instruction,
	timing: Instant,
}

impl Processor for ProgramState
{
	fn clock_speed(&self) -> u64 {
		1790000 /* 1.79 MHz */
	}

	fn run_time_with_start<F,U>(&mut self, start_time: Instant, cycles:u32, f: F) -> U where
		F: FnOnce(&mut Self) -> U,
	{
		let result = f(self);
		let ns = (1e9 as u64)*(cycles as u64)/(self.clock_speed());
		let frame_duration = time::Duration::from_nanos(ns);
		if(start_time.elapsed() > 3*frame_duration) {
			println!("YOOO TOOK AT LEAST THREE TIMES LONGER THAN EXPECTED ({} cycles, pc={}, instruction={:?}): {}x)", cycles, self.program_counter, self.current_instruction, start_time.elapsed().as_nanos()/frame_duration.as_nanos());
		}
		std::thread::sleep(frame_duration.saturating_sub(start_time.elapsed()));

		result
	}
}

impl ProgramState
{
	/* TODO comment */
	pub fn from_rom(rom: &Rom) -> Box<Self> {
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
			memory: CoreMemory::new(memory),
			current_instruction: Instruction::NOP,
			instruction_counter: 0,
			timing: Instant::now(),
		};

		result.program_counter = AddressingMode::Indirect.resolve_address_u16(&result, INITIAL_PC_LOCATION);

		Box::new(result)
	}

	pub fn transition(&mut self) {
		let start_time = Instant::now();
		if(self.memory.nmi_triggered()) {
			self.trigger_nmi();
		}

		let operation_loc = self.program_counter;
		/* TODO: what if this hits the top of program memory */
		// println!("operation loc 0x{operation_loc:x}");
		let operation = self.operation_from_memory(operation_loc);
		// match operation.realized_instruction.instruction {
			// _ => { println!("Running operation #{}, pc=0x{:x}: {:?}",
			// 				self.instruction_counter, self.program_counter, operation) }
		// }

		self.run_time_with_start(start_time, operation.realized_instruction.cycles as u32, |state| {
			state.current_instruction = operation.realized_instruction.instruction.clone();
			operation.apply(state);
			match operation.realized_instruction.instruction {
				// Instruction::RTI => {
				// 	println!("NMI TOTAL TIME: {}ms", state.timing.elapsed().as_millis());
				// }
				// Instruction::JSR => {
				// 	println!("JSR CALLED IN NMI, instruction count = {}", state.instruction_counter);
				// }
				// Instruction::RTS => {
				// 	println!("RTS CALLED IN NMI, instruction count = {}", state.instruction_counter);
				// }
				// Instruction::ROR => {
				// 	println!("ROR CALLED IN NMI, instruction count = {}", state.instruction_counter);
				// }
				// Instruction::BCC => {
				// 	println!("HEY BCC, instruction count = {}", state.instruction_counter);
				// }
				_ => {}
			}
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
		let value = self.memory.read(0x0100 + self.s_register as u16);
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
		let result = self.memory.read(addr);
		// if addr == 0x03ad {
		// 	println!("READING PLAYER REL X POS: 0x{:x}, pc = 0x{:x}", result, self.program_counter);
		// }
		// if addr == 0x03b8 {
		// 	println!("READING PLAYER REL Y POS: 0x{:x}, pc = 0x{:x}", result, self.program_counter);
		// }
		// println!("READING MEM");
		result
	}

	pub fn write_mem(&mut self, addr: u16, data: u8) {
		// if addr == 0xce {
		// 	println!("WRITING player_y: 0x{data:x}");
		// 	println!("-- program counter: 0x{:x}", self.program_counter);
		// }
		// if addr == 0x0770 {
		// 	println!("WRITING OperMode: 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// if addr == 0x03ad {
		// 	println!("WRITING PLAYER REL X POS: 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// if addr == 0x03b8 {
		// 	println!("WRITING PLAYER REL Y POS: 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// if addr == 0x0204 {
		// 	println!("WRITING TO 0x0204, BUFFER PLAYER SPRITE Y POS 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// if addr == 0x0776 {
		// 	println!("WRITING TO GAME PAUSE STATUS 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// if addr == 0x03d0 {
		// 	println!("WRITING PLAYER OFFSCREEN BITS: 0x{:x}, pc = 0x{:x}", data, self.program_counter);
		// }
		// println!("WRITING MEM");
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

	pub fn register_listener(&mut self, listener: PPUListener) {
		self.memory.register_listener(listener);
	}
	
	pub fn clone_memory(&self) -> CoreMemory {
		self.memory.clone()
	}

	fn trigger_nmi(&mut self) {
		println!("nmi triggered (instruction count = {})", self.instruction_counter);
		self.memory.reset_nmi();
		/* push PC onto stack */
		self.push_memory_loc(self.program_counter);
		/* push processor status register on stack */
		self.push(self.status);
		/* read NMI handler address from FFFA/FFFB and jump to that address*/
		self.program_counter = AddressingMode::Indirect.resolve_address_u16(&self, 0xfffa);
		self.timing = Instant::now();
	}

	fn operation_from_memory(&self, addr: u16) -> Operation
	{
		let mut data = [0; 3];
		self.memory.copy_range(addr as usize, &mut data);
		Operation {
			realized_instruction: from_opcode(data[0]),
			byte1: data[1],
			byte2: data[2],
		}
	}
}