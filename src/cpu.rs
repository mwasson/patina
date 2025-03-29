/* the state of the cpu at a given time */

pub struct ProgramState
{
	accumulator: u8,
	index_x: u8,
	index_y: u8,
	status: u8,
	memory: [u8; 1<<15]
}

pub enum Instruction
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
	IndexedIndirect,
	IndirectIndexed,
}

impl AddressingMode
{
	/* behavior based on: https://www.nesdev.org/obelisk-6502-guide/addressing.html */
	fn resolve_address(self: AddressingMode, state: ProgramState, byte1:u8, byte2:u8) -> usize {
		match self  {
			AddressingMode::Implicit =>
				panic!("Should never be explicitly referenced--remove?"),
			AddressingMode::Accumulator =>
				panic!("Should never be explicitly referenced--remove?"),
			AddressingMode::Immediate => 
				byte1 as usize,
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
            AddressingMode::IndexedIndirect => {
				let jmp_loc = zero_page_addr(byte1 + state.index_x);
				return addr(state.memory[jmp_loc+1], state.memory[jmp_loc]);
			}
			AddressingMode::IndirectIndexed => {
				let jmp_loc = zero_page_addr(byte1 + state.index_y);
				return addr(state.memory[jmp_loc+1], state.memory[jmp_loc]);
			}	
		}
	}
}



fn from_opcode(opcode: u8) -> Instruction {
	match opcode {
		0xa5 => Instruction::LDA, /* Zero Page, 2 bytes, 3 cycles */
		0xa9 => Instruction::LDA, /* #Immediate, 2 bytes, 2 cycles */
		0xa1 => Instruction::LDA, /* (Indirect,X), 2 bytes, 6 cycles */
		0xad => Instruction::LDA, /* Absolute, 3 bytes, 4 cycles */
		0xb1 => Instruction::LDA, /* (Indirect),Y, 2 bytes, 5/6 cycles */
		0xb5 => Instruction::LDA, /* Zero Page,X, 2 bytes, 4 cycles */
		0xb9 => Instruction::LDA, /* Absolute,Y, 3 bytes, 4/5 cycles */ 
		0xbd => Instruction::LDA, /* Absolute,X, 3 bytes, 4/5 cycles */
		_ => panic!("Unknown opcode")
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
