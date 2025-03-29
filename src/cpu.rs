/* the state of the cpu at a given time */

pub struct Cpu
{
	accumulator: u8,
	index_x: u8,
	index_y: u8,
	status: u8

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

/*impl Instruction
{
	fn opcode(self) -> u8 {
		match self {
			LDA => 0x00,
			_ => 0x00
		}		
	}
}*/

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

/* TODO implementation of Instruction behavior */
