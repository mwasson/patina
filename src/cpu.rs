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

	/* comparisons */
	CMP, /* Compare A */
	CPX, /* Compare X */
	CPY, /* Compare Y */

	/* branch codes */
	BCC, /* Branch if Carry Clear */
	BCS, /* Branch if Carry Set */
	BEQ, /* Branch if Equal */
	BMI, /* Branch if Minus */
	BNE, /* Branch if Not Equal */
	BPL, /* Branch if Plus */
	BVC, /* Branch if Overflow Clear */
	BVS, /* Branch if Overflow Set */

	/* increment/decrement locations */
	DEC, /* Decrement Memory */
	DEX, /* Decrement X */
	DEY, /* Decrement Y */
	INC, /* Increment Memory */
	INX, /* Increment X */
	INY, /* Increment Y */

	/* bitwise operators */
	EOR, /* Bitwise XOR */
	ORA, /* Bitwise OR */

    /* TODO others */
	BRK, /* Break (software IRQ) */
	CLD, /* Clear Decimal */
	SEI, /* Set InterruptDisable */

	/* jumps */
	JMP, /* Jump */
	JSR, /* Jump to Subroutine */
}

impl Mnemonic
{
	fn apply(self: Mnemonic, state: &mut ProgramState,
	         addr_mode: AddressingMode, b1: u8, b2: u8) {
		match self {
			Mnemonic::BCC => {
				Self::branch_instr(state, StatusFlag::Carry, false, b1)
			}	
			Mnemonic::BCS => {
				Self::branch_instr(state, StatusFlag::Carry, true, b1)
			}
			Mnemonic::BEQ => {
				Self::branch_instr(state, StatusFlag::Zero, true, b1)
			}
			Mnemonic::BMI => {
				Self::branch_instr(state, StatusFlag::Negative, true, b1)	
			}
			Mnemonic::BNE => {
				Self::branch_instr(state, StatusFlag::Zero, false, b1)
			}
			Mnemonic::BPL => {
				Self::branch_instr(state, StatusFlag::Negative, false, b1)
			}
			Mnemonic::BRK => {
				state.irq_with_offset(2);
			}
			Mnemonic::BVC => {
				Self::branch_instr(state, StatusFlag::Overflow, false, b1)
			}
			Mnemonic::BVS => {
				Self::branch_instr(state, StatusFlag::Overflow, true, b1)	
			}
			Mnemonic::CLD => {
				state.update_flag(StatusFlag::Decimal, false);
			}
			Mnemonic::CMP => {
				Self::compare(state, addr_mode, b1, b2, state.accumulator);
			}
			Mnemonic::CPX => {
				Self::compare(state, addr_mode, b1, b2, state.index_x);
			}
			Mnemonic::CPY => {
				Self::compare(state, addr_mode, b1, b2, state.index_y);
			}
			Mnemonic::DEC => {
				let new_val = addr_mode.deref(state, b1, b2) - 1;
				addr_mode.write(state, b1, b2, new_val);
				state.update_zero_neg_flags(new_val);
			}
			Mnemonic::DEX => {
				state.index_x -= 1;
				state.update_zero_neg_flags(state.index_x);
			}
			Mnemonic::DEY => {
				state.index_y -= 1;
				state.update_zero_neg_flags(state.index_y);
			}
			Mnemonic::EOR => {
				let mem_val = addr_mode.deref(state, b1, b2);
				state.accumulator = state.accumulator ^ mem_val;
				state.update_zero_neg_flags(state.accumulator);
			}
			Mnemonic::INC => {
				let new_val = addr_mode.deref(state, b1, b2) + 1;
				addr_mode.write(state, b1, b2, new_val);
				state.update_zero_neg_flags(new_val);
			}
			Mnemonic::INX => {
				state.index_x += 1;
				state.update_zero_neg_flags(state.index_x);
			}
			Mnemonic::INY => {
				state.index_y += 1;
				state.update_zero_neg_flags(state.index_y);
			}
			Mnemonic::JMP => {
				/* TODO: if this directly sets PC to the value in memory,
				 * does this imply other things that set PC need an offset? */
				state.program_counter = addr_mode.resolve_address(state,b1,b2);
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
			Mnemonic::LDY => {
				state.index_y = addr_mode.deref(state, b1, b2)
			}
			Mnemonic::ORA => {
				state.accumulator |= addr_mode.deref(state, b1, b2)
			}
			Mnemonic::SEI => {
				/* TODO: The effect is delayed "one instruction".
				 * Does that mean one cycle, or until the next instruction?
				 * how to implement this?
				 */
				state.update_flag(StatusFlag::InterruptDisable, true);
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

	fn branch_instr(state: &mut ProgramState, flag: StatusFlag, 
	                is_positive: bool, offset: u8) {
		if is_positive == flag.is_set(state) {
			state.program_counter = AddressingMode::Relative.resolve_address(
			                        state, offset, 0);
		}
	}

	fn compare(state: &mut ProgramState, addr_mode: AddressingMode,
	           b1: u8, b2: u8,
	           compare_val: u8) {
		let mem_val = addr_mode.deref(state, b1, b2);

		state.update_flag(StatusFlag::Carry, compare_val >= mem_val);
		state.update_zero_neg_flags(compare_val - mem_val);
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
	fn resolve_address(self: &AddressingMode, state: &mut ProgramState, byte1:u8, byte2:u8) -> u16 {
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

	fn deref(self: &AddressingMode, state: &mut ProgramState, byte1:u8, byte2:u8) -> u8 {
		match self {
			AddressingMode::Immediate => byte1,
			_ => {
				let address = self.resolve_address(state, byte1, byte2);
				state.mem_lookup(address)
			}
		}
	}

	fn write(self: &AddressingMode, state: &mut ProgramState, byte1: u8, byte2: u8, new_val: u8) {
		state.memory[self.resolve_address(state, byte1, byte2) as usize] = new_val;
	}
}



pub fn from_opcode(opcode: u8, b1: u8, b2: u8) -> Instruction {
	let (mnemonic, addr_mode, cycles, bytes) = match opcode {
		/* TODO: instructions marked 'boundary' take longer if crossing
		 * a page boundary */
		/* branch instructions also take an extra cycle if branch taken */

		0x00 => (Mnemonic::BRK, AddressingMode::Implicit, 7, 2),
		0x01 => (Mnemonic::ORA, AddressingMode::IndirectX, 6, 2),
		0x05 => (Mnemonic::ORA, AddressingMode::ZeroPage, 3, 2),
		0x09 => (Mnemonic::ORA, AddressingMode::Immediate, 2, 2),
		0x0d => (Mnemonic::ORA, AddressingMode::Absolute, 4, 3),
		0x10 => (Mnemonic::BPL, AddressingMode::Relative, 2, 2), /*boundary*/
		0x11 => (Mnemonic::ORA, AddressingMode::IndirectY, 5, 2), /*boundary*/
		0x15 => (Mnemonic::ORA, AddressingMode::ZeroPageX, 3, 2),
		0x19 => (Mnemonic::ORA, AddressingMode::AbsoluteY, 4, 3), /*boundary*/
		0x1d => (Mnemonic::ORA, AddressingMode::AbsoluteX, 4, 3), /*boundary*/
		0x20 => (Mnemonic::JSR, AddressingMode::Absolute, 6, 3),
		0x30 => (Mnemonic::BMI, AddressingMode::Relative, 2, 2), /*boundary*/
		0x41 => (Mnemonic::EOR, AddressingMode::IndirectX, 6, 2),
		0x45 => (Mnemonic::EOR, AddressingMode::ZeroPage, 3, 2),
		0x49 => (Mnemonic::EOR, AddressingMode::Immediate, 2, 2),
		0x4c => (Mnemonic::JMP, AddressingMode::Absolute, 3, 3),
		0x4d => (Mnemonic::EOR, AddressingMode::Absolute, 4, 3),
		0x50 => (Mnemonic::BVC, AddressingMode::Relative, 2, 2), /*boundary*/
		0x51 => (Mnemonic::EOR, AddressingMode::IndirectY, 5, 2), /*boundary*/
		0x55 => (Mnemonic::EOR, AddressingMode::ZeroPageX, 4, 2),
		0x59 => (Mnemonic::EOR, AddressingMode::AbsoluteY, 4, 3), /*boundary*/
		0x5d => (Mnemonic::EOR, AddressingMode::AbsoluteX, 4, 3), /*boundary*/
		0x6c => (Mnemonic::JMP, AddressingMode::Indirect, 5, 3),
		0x70 => (Mnemonic::BVS, AddressingMode::Relative, 2, 2), /*boundary*/
		0x78 => (Mnemonic::SEI, AddressingMode::Implicit, 2, 1),
		0x81 => (Mnemonic::STA, AddressingMode::IndirectX, 6, 2),
		0x85 => (Mnemonic::STA, AddressingMode::ZeroPage, 3, 2),
		0x88 => (Mnemonic::DEY, AddressingMode::Implicit, 2, 1),
		0x8d => (Mnemonic::STA, AddressingMode::Absolute, 4, 3),
		0x90 => (Mnemonic::BCC, AddressingMode::Relative, 2, 2), /*boundary*/
		0x91 => (Mnemonic::STA, AddressingMode::IndirectY, 6, 2),
		0x95 => (Mnemonic::STA, AddressingMode::ZeroPageX, 4, 3),
		0x99 => (Mnemonic::STA, AddressingMode::AbsoluteY, 5, 3),
		0x9a => (Mnemonic::TXS, AddressingMode::Implicit, 2, 1),
		0x9d => (Mnemonic::STA, AddressingMode::AbsoluteX, 5, 3),
		0xa0 => (Mnemonic::LDY, AddressingMode::Immediate, 2, 2),
		0xa2 => (Mnemonic::LDX, AddressingMode::Immediate, 2, 2),
		0xa4 => (Mnemonic::LDY, AddressingMode::ZeroPage, 3, 2),
		0xa5 => (Mnemonic::LDA, AddressingMode::ZeroPage, 3, 2),
		0xa6 => (Mnemonic::LDX, AddressingMode::ZeroPage, 3, 2),
		0xa9 => (Mnemonic::LDA, AddressingMode::Immediate, 2, 2),
		0xa1 => (Mnemonic::LDA, AddressingMode::IndirectX, 6, 2),
		0xac => (Mnemonic::LDY, AddressingMode::Absolute, 4, 3),
		0xad => (Mnemonic::LDA, AddressingMode::Absolute, 4, 3),
		0xae => (Mnemonic::LDX, AddressingMode::Absolute, 4, 3),
		0xb0 => (Mnemonic::BCS, AddressingMode::Relative, 3, 2), /*boundary*/
		0xb1 => (Mnemonic::LDA, AddressingMode::IndirectY, 5, 2), /*boundary*/
		0xb4 => (Mnemonic::LDY, AddressingMode::ZeroPageX, 4, 2),
		0xb5 => (Mnemonic::LDA, AddressingMode::ZeroPageY, 4, 2),
		0xb6 => (Mnemonic::LDX, AddressingMode::ZeroPageY, 4, 2),
		0xb9 => (Mnemonic::LDA, AddressingMode::AbsoluteY, 4, 3), /*boundary*/
		0xbc => (Mnemonic::LDY, AddressingMode::AbsoluteX, 4, 3), /*boundary*/
		0xbd => (Mnemonic::LDA, AddressingMode::AbsoluteX, 4, 3), /*boundary*/
		0xbe => (Mnemonic::LDX, AddressingMode::AbsoluteY, 4, 3), /*boundary*/
		0xc0 => (Mnemonic::CPY, AddressingMode::Immediate, 2, 2),
		0xc1 => (Mnemonic::CMP, AddressingMode::IndirectX, 6, 2),
		0xc4 => (Mnemonic::CPY, AddressingMode::ZeroPage, 3, 2),
		0xc5 => (Mnemonic::CMP, AddressingMode::ZeroPage, 3, 2),
		0xc6 => (Mnemonic::DEC, AddressingMode::ZeroPage, 5, 2),
		0xc8 => (Mnemonic::INY, AddressingMode::Implicit, 2, 1),
		0xc9 => (Mnemonic::CMP, AddressingMode::Immediate, 2, 2),
		0xca => (Mnemonic::DEX, AddressingMode::Implicit, 2, 1),
		0xcc => (Mnemonic::CPY, AddressingMode::Absolute, 4, 3),
		0xcd => (Mnemonic::CMP, AddressingMode::Absolute, 4, 3),
		0xce => (Mnemonic::DEC, AddressingMode::Absolute, 6, 3),
		0xd0 => (Mnemonic::BNE, AddressingMode::Relative, 2, 2), /*boundary*/
		0xd1 => (Mnemonic::CMP, AddressingMode::IndirectY, 5, 2), /*boundary*/
		0xd5 => (Mnemonic::CMP, AddressingMode::ZeroPageX, 4, 2),
		0xd6 => (Mnemonic::DEC, AddressingMode::ZeroPageX, 6, 2),
		0xd8 => (Mnemonic::CLD, AddressingMode::Implicit, 2, 1),
		0xd9 => (Mnemonic::CMP, AddressingMode::AbsoluteY, 4, 3), /*boundary*/
		0xdd => (Mnemonic::CMP, AddressingMode::AbsoluteX, 4, 3), /*boundary*/
		0xde => (Mnemonic::DEC, AddressingMode::AbsoluteX, 7, 3),
		0xe0 => (Mnemonic::CPX, AddressingMode::Immediate, 2, 2),
		0xe4 => (Mnemonic::CPX, AddressingMode::ZeroPage, 3, 2),
		0xe6 => (Mnemonic::INC, AddressingMode::ZeroPage, 5, 2),
		0xe8 => (Mnemonic::INX, AddressingMode::Implicit, 2, 1),
		0xec => (Mnemonic::CPX, AddressingMode::Absolute, 4, 3),
		0xee => (Mnemonic::INC, AddressingMode::Absolute, 6, 3),
		0xf0 => (Mnemonic::BEQ, AddressingMode::Relative, 2, 2), /*boundary*/
		0xf6 => (Mnemonic::INC, AddressingMode::ZeroPageX, 6, 3),
		0xfe => (Mnemonic::INC, AddressingMode::AbsoluteX, 7, 3),
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

	pub fn is_set(self, state: &ProgramState) -> bool {
		state.status & self.mask() != 0
	}

	pub fn update_bool(self, state: &mut ProgramState, new_val: bool) {
		let new_val_as_number = if new_val { 1 } else { 0 };
		self.update(state, new_val_as_number);
	}

	pub fn update(self, state: &mut ProgramState, new_val: u8) {
		state.status = state.status & (new_val << self.mask());
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
 * Zero page address operations take a single-byte and result in an
 * address on the first page of memory, which has addresses that begin
 * with 0x00. If this is passed in 0xAB, it returns 0x00AB. In effect this
 * is just a cast, but wrapping it as a function makes the goal clearer.
 */
fn zero_page_addr(b1:u8) -> u16 {
	b1 as u16
}
