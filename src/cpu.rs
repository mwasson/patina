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

/* TODO implementation of Instruction behavior */
