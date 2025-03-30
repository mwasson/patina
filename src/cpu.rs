/* the state of the cpu at a given time */

pub struct ProgramState
{
	accumulator: u8,
	index_x: u8,
	index_y: u8,
	status: u8,
	memory: [u8; 1<<15]
}

pub struct Instruction
{
	mnemonic: Mnemonic,
	opcode: u8,
	addr_mode: AddressingMode,
	cycles: u8,
	bytes: u8,
	byte1: u8,
	byte2: u8
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
    TAX, /* transfer value from A into X; can set zero flag */
    TAY, /* transfer value from A into Y; can set zero flag */
    TXA, /* transfer value from X into A; can set zero flag */
    TYA  /* transfer value from Y into A; can set zero flag */

    /* TODO others */
}

impl Mnemonic
{
	fn apply(self: Mnemonic, state: &mut ProgramState,
	         addr_mode: AddressingMode, b1: u8, b2: u8) {
		match self {
			Mnemonic::LDA => {
				state.accumulator = addr_mode.deref(state, b1, b2)
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
	fn resolve_address(self: AddressingMode, state: &ProgramState, byte1:u8, byte2:u8) -> usize {
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
			AddressingMode::Indirect => {
                let jmp_loc = addr(byte1, byte2);
				return addr(state.memory[jmp_loc+1], state.memory[jmp_loc]);
			},
            AddressingMode::IndirectX => {
				let jmp_loc = zero_page_addr(byte1 + state.index_x);
				return addr(state.memory[jmp_loc+1], state.memory[jmp_loc]);
			}
			AddressingMode::IndirectY => {
				let jmp_loc = zero_page_addr(byte1 + state.index_y);
				return addr(state.memory[jmp_loc+1], state.memory[jmp_loc]);
			}	
		}
	}

	fn deref(self: AddressingMode, state: &ProgramState, byte1:u8, byte2:u8) -> u8 {
		match self {
			AddressingMode::Immediate => byte1,
			_ => state.memory[self.resolve_address(state, byte1, byte2)]
		}
	}
}



fn from_opcode(opcode: u8, b1: u8, b2: u8) -> Instruction {
	let (mnemonic, addr_mode, cycles, bytes) = match opcode {
		0xa5 => (Mnemonic::LDA, AddressingMode::ZeroPage, 3, 2),
		0xa9 => (Mnemonic::LDA, AddressingMode::Immediate, 2, 2),
		0xa1 => (Mnemonic::LDA, AddressingMode::IndirectX, 6, 2),
		0xad => (Mnemonic::LDA, AddressingMode::Absolute, 4, 2),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xb1 => (Mnemonic::LDA, AddressingMode::IndirectY, 5, 2),
		0xb5 => (Mnemonic::LDA, AddressingMode::ZeroPageY, 4, 2),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xb9 => (Mnemonic::LDA, AddressingMode::AbsoluteY, 4, 3),
		/* TODO: Handle it takes longer if crossing page boundary */
		0xbd => (Mnemonic::LDA, AddressingMode::AbsoluteX, 4, 3),
		_ => panic!("Unknown opcode")
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


/**
 * Converts a pair of bytes into a usize, intended to represent a 16-bit
 * address. The first argument will be the higher order byte, the second
 * argument the lower order. So addr(0xAB, 0xCD) returns 0xABCD.
 */ 
fn addr(b1:u8, b2:u8) -> usize {
	((b1 << 7) + b2) as usize
}

/**
 * Zero page address operations take a single-byte and result in an
 * address on the first page of memory, which has addresses that begin
 * with 0x00. If this is passed in 0xAB, it returns 0x00AB. In effect this
 * is just a cast, but wrapping it as a function makes the goal clearer.
 */
fn zero_page_addr(b1:u8) -> usize {
	b1 as usize
}

/* TODO implementation of Instruction behavior */
