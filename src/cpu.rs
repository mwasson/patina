/* the state of the cpu at a given time */

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

	pub fn update_flag(&mut self, flag: StatusFlag, new_val: u8) {
		flag.update(self, new_val);
	}

	pub fn push(&mut self, data: u8) {
		self.memory[addr(0x10, self.s_register) as usize] = data;
		self.s_register -= 1;
	}

	pub fn push_memory_loc(&mut self, mem_loc: u16) {
		self.push((mem_loc >> 8) as u8);
		self.push(mem_loc as u8);
	}

	pub fn pop_memory_loc(&mut self) -> u16 {
		let lower = self.pop();
		let upper = self.pop();

		addr(upper, lower)
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
		self.update_flag(StatusFlag::InterruptDisable, 0);
		/* TODO: jump to IRQ handler */
	}

	pub fn mem_lookup(&mut self, addr: u16) -> u8 {
		self.memory[addr as usize]
	}

	pub fn addr_from_mem(&mut self, addr_to_lookup: u16) -> u16 {
		addr(self.mem_lookup(addr_to_lookup+1), self.mem_lookup(addr_to_lookup))
	}
}

pub struct Instruction
{
	pub mnemonic: Mnemonic,
	pub opcode: u8,
	pub addr_mode: AddressingMode,
	pub cycles: u8,
	pub bytes: u8,
	pub byte1: u8,
	pub byte2: u8
}

pub enum Mnemonic
{
    /* load/store opcodes */
    LDA, /* loads fixed value into A; can set zero flag */
    LDX, /* loads value at address into X; can set zero flag */
    LDY, /* loads fixed value into Y; can set zero flag */
    STA, /* store value from A into address */
    STX, /* stores value from X into address */
    STY, /* stores value from Y into address */

	/* transfer opcodes */
    TAX, /* transfer value from A into X; can set zero flag */
    TAY, /* transfer value from A into Y; can set zero flag */
	TXS, /* Transfer X to Stack Pointer */
    TXA, /* transfer value from X into A; can set zero flag */
    TYA,  /* transfer value from Y into A; can set zero flag */

    /* TODO others */
	BRK, /* Break (software IRQ) */
	CLD, /* Clear Decimal */
	SEI, /* Set InterruptDisable */

	JSR, /* Jump to Subroutine */
}

impl Mnemonic
{
	fn apply(self: Mnemonic, state: &mut ProgramState,
	         addr_mode: AddressingMode, b1: u8, b2: u8) {
		match self {
			Mnemonic::BRK => {
				state.irq_with_offset(2);
			}
			Mnemonic::CLD => {
				state.update_flag(StatusFlag::Decimal, 0);
			}
			Mnemonic::JSR => {
				state.push_memory_loc(state.program_counter + 2);
				state.program_counter = addr_mode.resolve_address(state,b1,b2);
			}
			Mnemonic::LDA => {
				state.accumulator = addr_mode.deref(state, b1, b2)
			}
			Mnemonic::LDX => {
				state.index_x = addr_mode.deref(state, b1, b2)
			}
			Mnemonic::SEI => {
				/* TODO: The effect is delayed "one instruction".
				 * Does that mean one cycle, or until the next instruction?
				 * how to implement this?
				 */
				state.update_flag(StatusFlag::InterruptDisable, 1);
			}
			Mnemonic::STA => {
				addr_mode.write(state, b1, b2, state.accumulator);
			}
			Mnemonic::TXS => {
				state.s_register = state.index_x
			}
			_ => panic!("Unimplemented")
		}
	}
}

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
	fn resolve_address(self: AddressingMode, state: &mut ProgramState, byte1:u8, byte2:u8) -> u16 {
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
				panic!("Relative unimplemented for now"),
			AddressingMode::Absolute =>
				addr(byte1, byte2),
			AddressingMode::AbsoluteX =>
				addr(byte1, byte2 + state.index_x),
			AddressingMode::AbsoluteY =>
				addr(byte1, byte2 + state.index_y),
			AddressingMode::Indirect =>
                state.addr_from_mem(addr(byte1, byte2)),
            AddressingMode::IndirectX =>
				state.addr_from_mem(zero_page_addr(byte1 + state.index_x)),
			AddressingMode::IndirectY =>
				state.addr_from_mem(zero_page_addr(byte1 + state.index_y)),
		}
	}

	fn deref(self: AddressingMode, state: &mut ProgramState, byte1:u8, byte2:u8) -> u8 {
		match self {
			AddressingMode::Immediate => byte1,
			_ => {
				let address = self.resolve_address(state, byte1, byte2);
				state.mem_lookup(address)
			}
		}
	}

	fn write(self: AddressingMode, state: &mut ProgramState, byte1: u8, byte2: u8, new_val: u8) {
		state.memory[self.resolve_address(state, byte1, byte2) as usize] = new_val;
	}
}



pub fn from_opcode(opcode: u8, b1: u8, b2: u8) -> Instruction {
	let (mnemonic, addr_mode, cycles, bytes) = match opcode {
		0x00 => (Mnemonic::BRK, AddressingMode::Implicit, 7, 2),
		0x20 => (Mnemonic::JSR, AddressingMode::Absolute, 6, 3),
		0x78 => (Mnemonic::SEI, AddressingMode::Implicit, 2, 1),
		0x81 => (Mnemonic::STA, AddressingMode::IndirectX, 6, 2),
		0x85 => (Mnemonic::STA, AddressingMode::ZeroPage, 3, 2),
		0x8d => (Mnemonic::STA, AddressingMode::Absolute, 4, 3),
		0x91 => (Mnemonic::STA, AddressingMode::IndirectY, 6, 2),
		0x95 => (Mnemonic::STA, AddressingMode::ZeroPageX, 4, 3),
		0x99 => (Mnemonic::STA, AddressingMode::AbsoluteY, 5, 3),
		0x9a => (Mnemonic::TXS, AddressingMode::Implicit, 2, 1),
		0x9d => (Mnemonic::STA, AddressingMode::AbsoluteX, 5, 3),
		0xa2 => (Mnemonic::LDX, AddressingMode::Immediate, 2, 2),
		0xa5 => (Mnemonic::LDA, AddressingMode::ZeroPage, 3, 2),
		0xa6 => (Mnemonic::LDX, AddressingMode::ZeroPage, 3, 2),
		0xa9 => (Mnemonic::LDA, AddressingMode::Immediate, 2, 2),
		0xa1 => (Mnemonic::LDA, AddressingMode::IndirectX, 6, 2),
		0xad => (Mnemonic::LDA, AddressingMode::Absolute, 4, 2),
		0xae => (Mnemonic::LDX, AddressingMode::Absolute, 4, 3),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xb1 => (Mnemonic::LDA, AddressingMode::IndirectY, 5, 2),
		0xb5 => (Mnemonic::LDA, AddressingMode::ZeroPageY, 4, 2),
		0xb6 => (Mnemonic::LDX, AddressingMode::ZeroPageY, 4, 2),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xb9 => (Mnemonic::LDA, AddressingMode::AbsoluteY, 4, 3),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xbd => (Mnemonic::LDA, AddressingMode::AbsoluteX, 4, 3),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xbe => (Mnemonic::LDX, AddressingMode::AbsoluteY, 4, 3),		
		0xd8 => (Mnemonic::CLD, AddressingMode::Implicit, 2, 1),
		_ => panic!("Unknown opcode 0x{opcode:x}")
	};

	Instruction {
    	mnemonic: mnemonic, 
    	opcode: opcode, 
    	addr_mode: addr_mode,
    	cycles: cycles,
    	bytes: bytes,
    	byte1: b1,
    	byte2: b2
	}
}

pub enum StatusFlag
{
	Carry,
	Zero,
	InterruptDisable,
	Decimal,
	/* "No CPU effect; see: the B flag" */
	/* "No CPU effect; always pushed as 1" */
	Overflow,
	Negative
}

impl StatusFlag
{
	pub fn mask(self) -> u8 {
		match self {
			StatusFlag::Carry => 0,
			StatusFlag::Zero => 1,
			StatusFlag::InterruptDisable => 2,
			StatusFlag::Decimal => 3,
			StatusFlag::Overflow => 6,
			StatusFlag::Negative => 7
		}
	}

	pub fn update(self, state: &mut ProgramState, new_val: u8) {
		state.status = state.status & (new_val << self.mask());
	}
}

/**
 * Converts a pair of bytes into a u16 to look up an address in memory.
 * The first argument will be the higher order byte, the second
 * argument the lower order. So addr(0xAB, 0xCD) returns 0xABCD.
 */ 
fn addr(b1:u8, b2:u8) -> u16 {
	((b1 as u16) << 8) + (b2 as u16)
}

/**
 * Zero page address operations take a single-byte and result in an
 * address on the first page of memory, which has addresses that begin
 * with 0x00. If this is passed in 0xAB, it returns 0x00AB. In effect this
 * is just a cast, but wrapping it as a function makes the goal clearer.
 */
fn zero_page_addr(b1:u8) -> u16 {
	b1 as u16
}
