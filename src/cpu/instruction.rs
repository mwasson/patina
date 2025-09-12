use crate::cpu::{AddressingMode, StatusFlag};

use AddressingMode::*;
use crate::cpu::program_state::ProgramState;

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction
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
	TSX, /* transfer value from Stack Pointer to X; can set zero flag */
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
	AND, /* Bitwise AND */
	ASL, /* Arithmetic Shift Left */
	BIT, /* Bit Test */
	EOR, /* Bitwise XOR */
	LSR, /* Logical Shift Right */
	ORA, /* Bitwise OR */

	/* arithmetic */
	ADC, /* Add With Carry */
	SBC, /* Subtract With Carry */

	/* rotates */
	ROL, /* Rotate Left */
	ROR, /* Rotate Right */

	/* clear & set flags */
	CLC, /* Clear Carry */
	CLD, /* Clear Decimal */
	SEC, /* Set Carry Flag */
	SEI, /* Set InterruptDisable */

	/* stack operations */
	PHA, /* Push A */
	PHP, /* Push Processor Status */
	PLA, /* Pull A */
	PLP, /* Pull Processor Status */

	/* jumps */
	JMP, /* Jump */
	JSR, /* Jump to Subroutine */
	RTS, /* Return from Subroutine */
	RTI, /* Return from Interrupt */

	/* others */
	BRK, /* Break (software IRQ) */
	NOP, /* No-op */
}

impl Instruction
{
	pub fn apply(&self, state: &mut ProgramState,
			 addr_mode: &AddressingMode, b1: u8, b2: u8) {
		match self {
			Instruction::ADC => {
				let val = addr_mode.deref(state, b1, b2);
				add_with_carry_and_update(state, val, StatusFlag::Carry.as_num(state));
			}
			Instruction::AND => {
				let mem_val = addr_mode.deref(state, b1, b2);
				state.accumulator = state.accumulator & mem_val;
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::ASL => {
				let old_val: u8 = addr_mode.deref(state, b1, b2);
				let result = old_val << 1;
				state.update_flag(StatusFlag::Carry, old_val & 0x80 != 0);
				state.update_zero_neg_flags(result);
				addr_mode.write(state, b1, b2, result);
			}
			Instruction::BCC => {
				Self::branch_instr(state, StatusFlag::Carry, false, b1)
			}
			Instruction::BCS => {
				Self::branch_instr(state, StatusFlag::Carry, true, b1)
			}
			Instruction::BEQ => {
				Self::branch_instr(state, StatusFlag::Zero, true, b1)
			}
			Instruction::BIT => {
				let mem = addr_mode.deref(state, b1, b2);
				let val = state.accumulator & mem;
				state.update_flag(StatusFlag::Zero, val == 0);
				state.update_flag(StatusFlag::Overflow, mem & 0x40 != 0);
				state.update_flag(StatusFlag::Negative, mem & 0x80 != 0);
			}
			Instruction::BMI => {
				Self::branch_instr(state, StatusFlag::Negative, true, b1)
			}
			Instruction::BNE => {
				Self::branch_instr(state, StatusFlag::Zero, false, b1)
			}
			Instruction::BPL => {
				Self::branch_instr(state, StatusFlag::Negative, false, b1)
			}
			Instruction::BRK => {
				state.irq_with_offset(2);
			}
			Instruction::BVC => {
				Self::branch_instr(state, StatusFlag::Overflow, false, b1)
			}
			Instruction::BVS => {
				Self::branch_instr(state, StatusFlag::Overflow, true, b1)
			}
			Instruction::CLC => {
				state.update_flag(StatusFlag::Carry, false);
			}
			Instruction::CLD => {
				state.update_flag(StatusFlag::Decimal, false);
			}
			Instruction::CMP => {
				Self::compare(state, addr_mode, b1, b2, state.accumulator);
			}
			Instruction::CPX => {
				Self::compare(state, addr_mode, b1, b2, state.index_x);
			}
			Instruction::CPY => {
				Self::compare(state, addr_mode, b1, b2, state.index_y);
			}
			Instruction::DEC => {
				let new_val = addr_mode.deref(state, b1, b2).wrapping_sub(1);
				addr_mode.write(state, b1, b2, new_val);
				state.update_zero_neg_flags(new_val);
			}
			Instruction::DEX => {
				state.index_x = state.index_x.wrapping_sub(1);
				state.update_zero_neg_flags(state.index_x);
			}
			Instruction::DEY => {
				state.index_y = state.index_y.wrapping_sub(1);
				state.update_zero_neg_flags(state.index_y);
			}
			Instruction::EOR => {
				let mem_val = addr_mode.deref(state, b1, b2);
				state.accumulator = state.accumulator ^ mem_val;
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::INC => {
				let new_val = addr_mode.deref(state, b1, b2).wrapping_add(1);
				addr_mode.write(state, b1, b2, new_val);
				state.update_zero_neg_flags(new_val);
			}
			Instruction::INX => {
				state.index_x = state.index_x.wrapping_add(1);
				state.update_zero_neg_flags(state.index_x);
			}
			Instruction::INY => {
				state.index_y = state.index_y.wrapping_add(1);
				state.update_zero_neg_flags(state.index_y);
			}
			Instruction::JMP => {
				/* TODO: if this directly sets PC to the value in memory,
				 * does this imply other things that set PC need an offset? */
				state.program_counter = addr_mode.resolve_address(state,b1,b2);
			}
			Instruction::JSR => {
				state.push_memory_loc(state.program_counter + 2);
				state.program_counter = addr_mode.resolve_address(state,b1,b2);
			}
			Instruction::LDA => {
				state.accumulator = addr_mode.deref(state, b1, b2);
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::LDX => {
				state.index_x = addr_mode.deref(state, b1, b2);
				state.update_zero_neg_flags(state.index_x);
			}
			Instruction::LDY => {
				state.index_y = addr_mode.deref(state, b1, b2);
				state.update_zero_neg_flags(state.index_y);
			}
			Instruction::LSR => {
				let val = addr_mode.deref(state, b1, b2);
				let new_val = val >> 1;
				addr_mode.write(state, b1, b2, new_val);
				state.update_flag(StatusFlag::Carry, (val & 0x1) != 0);
				state.update_flag(StatusFlag::Zero, new_val == 0);
				state.update_flag(StatusFlag::Negative, false);
			}
			Instruction::NOP => {
				/* nothing */
			}
			Instruction::ORA => {
				state.accumulator |= addr_mode.deref(state, b1, b2);
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::PHA => {
				state.write_mem(0x100 + state.s_register as u16, state.accumulator);
				state.s_register -= 1;
			}
			Instruction::PHP => {
				/* pushes status onto the stack, with the 'B' flag (bit 4) on */
				state.write_mem(0x100 + state.s_register as u16, state.status | (1<<4));
				state.s_register -= 1;

			}
			Instruction::PLA => {
				state.s_register += 1;
				state.accumulator = state.read_mem(0x100 + state.s_register as u16);
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::PLP => {
				/* reads status from the stack, except for bits 4 and 5 */
				state.s_register += 1;
				let val = state.read_mem(0x100 + state.s_register as u16);
				state.status = (state.status & 0x30) | (val & !0x30);
			}
			Instruction::ROL => {
				let val = addr_mode.deref(state, b1, b2);
				let mut result = StatusFlag::Carry.as_num(state);
				result = result | (val << 1);
				addr_mode.write(state, b1, b2, result);
				state.update_flag(StatusFlag::Carry, val & 0x80 != 0);
				state.update_zero_neg_flags(result);
			}
			Instruction::ROR => {
				let val = addr_mode.deref(state, b1, b2);
				let mut result = StatusFlag::Carry.as_num(state) << 7;
				result = result | (val >> 1);
				addr_mode.write(state, b1, b2, result);
				state.update_flag(StatusFlag::Carry, val & 0x1 != 0);
				state.update_zero_neg_flags(result);
			}
			Instruction::RTI => {
				state.status = state.pop();
				state.program_counter = state.pop_memory_loc();
			}
			Instruction::RTS => {
				state.program_counter = state.pop_memory_loc() + 1;
			}
			Instruction::SBC => {
				let val = addr_mode.deref(state, b1, b2);
				add_with_carry_and_update(state, !val, StatusFlag::Carry.as_num(state));
			}
			Instruction::SEC => {
				StatusFlag::Carry.update_bool(state, true);
			}
			Instruction::SEI => {
				/* TODO: The effect is delayed "one instruction".
				 * Does that mean one cycle, or until the next instruction?
				 * how to implement this?
				 */
				state.update_flag(StatusFlag::InterruptDisable, true);
			}
			Instruction::STA => {
				addr_mode.write(state, b1, b2, state.accumulator);
			}
			Instruction::STX => {
				addr_mode.write(state, b1, b2, state.index_x);
			}
			Instruction::STY => {
				addr_mode.write(state, b1, b2, state.index_y);
			}
			Instruction::TAX => {
				state.index_x = state.accumulator;
				state.update_zero_neg_flags(state.index_x);
			}
			Instruction::TAY => {
				state.index_y = state.accumulator;
				state.update_zero_neg_flags(state.index_y);
			}
			Instruction::TSX => {
				state.index_x = state.s_register;
				state.update_zero_neg_flags(state.index_x);
			}
			Instruction::TXA => {
				state.accumulator = state.index_x;
				state.update_zero_neg_flags(state.accumulator);
			}
			Instruction::TXS => {
				state.s_register = state.index_x;
				/* doesn't update flags! multiple sources agree on this ? */
			}
			Instruction::TYA => {
				state.accumulator = state.index_y;
				state.update_zero_neg_flags(state.accumulator);
			}
		}
	}

	fn branch_instr(state: &mut ProgramState, flag: StatusFlag,
	                is_positive: bool, offset: u8) {
		if is_positive == flag.is_set(state) {
			state.program_counter = Relative.resolve_address(
			                        state, offset, 0);
		}
	}

	fn compare(state: &mut ProgramState, addr_mode: &AddressingMode,
	           b1: u8, b2: u8,
	           compare_val: u8) {
		let mem_val = addr_mode.deref(state, b1, b2);

		state.update_flag(StatusFlag::Carry, compare_val >= mem_val);
		state.update_flag(StatusFlag::Zero, compare_val == mem_val);
		state.update_flag(StatusFlag::Negative, compare_val.wrapping_sub(mem_val) & 0x80 != 0);
	}
}

#[derive(Debug)]
pub struct RealizedInstruction
{
    pub instruction: Instruction,
    pub addr_mode: AddressingMode,
    pub cycles: u16,
    pub bytes: u8,
}

impl RealizedInstruction
{
	pub fn apply(&self, state: &mut ProgramState, b1: u8, b2: u8) {
		self.instruction.apply(state, &self.addr_mode, b1, b2);
		/* note that this holds even for branching instructions (but not jump instructions): program counter needs to be
		 * incremented by the number of bytes for instruction, arguments
		 */

		match &self.instruction {
			Instruction::JMP => {}
			Instruction::JSR => {}
			Instruction::RTS => {}
			Instruction::RTI => {}
			_ => {state.program_counter = state.program_counter.wrapping_add(self.bytes as u16);}
		}
	}
}

pub fn from_opcode(opcode: u8) -> RealizedInstruction {
	let (instruction, addr_mode, cycles, bytes) = match opcode {
		/* TODO: instructions marked 'boundary' take longer if crossing
		 * a page boundary */
		/* branch instructions also take an extra cycle if branch taken */

		0x00 => (Instruction::BRK, Implicit, 7, 2),
		0x06 => (Instruction::ASL, ZeroPage, 5, 2),
		0x08 => (Instruction::PHP, Implicit, 3, 1),
		0x0a => (Instruction::ASL, Accumulator, 2, 1),
		0x01 => (Instruction::ORA, IndirectX, 6, 2),
		0x05 => (Instruction::ORA, ZeroPage, 3, 2),
		0x09 => (Instruction::ORA, Immediate, 2, 2),
		0x0d => (Instruction::ORA, Absolute, 4, 3),
		0x0e => (Instruction::ASL, Absolute, 6, 3),
		0x10 => (Instruction::BPL, Relative, 2, 2), /*boundary*/
		0x11 => (Instruction::ORA, IndirectY, 5, 2), /*boundary*/
		0x15 => (Instruction::ORA, ZeroPageX, 3, 2),
		0x16 => (Instruction::ASL, ZeroPageX, 6, 2),
		0x18 => (Instruction::CLC, Implicit, 2, 1),
		0x19 => (Instruction::ORA, AbsoluteY, 4, 3), /*boundary*/
		0x1d => (Instruction::ORA, AbsoluteX, 4, 3), /*boundary*/
		0x1e => (Instruction::ASL, AbsoluteX, 7, 3),
		0x20 => (Instruction::JSR, Absolute, 6, 3),
		0x21 => (Instruction::AND, IndirectX, 6, 2),
		0x24 => (Instruction::BIT, ZeroPage, 3, 2),
		0x25 => (Instruction::AND, ZeroPage, 3, 2),
		0x26 => (Instruction::ROL, ZeroPage, 5, 2),
		0x28 => (Instruction::PLP, Implicit, 4, 1),
		0x29 => (Instruction::AND, Immediate, 2, 2),
		0x2a => (Instruction::ROL, Accumulator, 2, 1),
		0x2c => (Instruction::BIT, Absolute, 4, 3),
		0x2d => (Instruction::AND, Absolute, 4, 3),
		0x2e => (Instruction::ROL, Absolute, 6, 3),
		0x30 => (Instruction::BMI, Relative, 2, 2), /*boundary*/
		0x31 => (Instruction::AND, IndirectY, 5, 2), /*boundary*/
		0x35 => (Instruction::AND, ZeroPageX, 4, 2),
		0x36 => (Instruction::ROL, ZeroPageX, 6, 2),
		0x38 => (Instruction::SEC, Implicit, 2, 1),
		0x39 => (Instruction::AND, AbsoluteY, 4, 3), /*boundary*/
		0x3d => (Instruction::AND, AbsoluteX, 4, 3), /*boundary*/
		0x3e => (Instruction::ROL, AbsoluteX, 7, 3),
		0x40 => (Instruction::RTI, Implicit, 6, 1),
		0x41 => (Instruction::EOR, IndirectX, 6, 2),
		0x45 => (Instruction::EOR, ZeroPage, 3, 2),
		0x46 => (Instruction::LSR, ZeroPage, 5, 2),
		0x48 => (Instruction::PHA, Implicit, 3, 1),
		0x49 => (Instruction::EOR, Immediate, 2, 2),
		0x4a => (Instruction::LSR, Accumulator, 2, 1),
		0x4c => (Instruction::JMP, Absolute, 3, 3),
		0x4d => (Instruction::EOR, Absolute, 4, 3),
		0x4e => (Instruction::LSR, Absolute, 6, 3),
		0x50 => (Instruction::BVC, Relative, 2, 2), /*boundary*/
		0x51 => (Instruction::EOR, IndirectY, 5, 2), /*boundary*/
		0x55 => (Instruction::EOR, ZeroPageX, 4, 2),
		0x56 => (Instruction::LSR, ZeroPageX, 6, 2),
		0x59 => (Instruction::EOR, AbsoluteY, 4, 3), /*boundary*/
		0x5d => (Instruction::EOR, AbsoluteX, 4, 3), /*boundary*/
		0x5e => (Instruction::LSR, AbsoluteX, 7, 3),
		0x60 => (Instruction::RTS, Implicit, 6, 1),
		0x61 => (Instruction::ADC, IndirectX, 6, 2),
		0x65 => (Instruction::ADC, ZeroPage, 3, 2),
		0x66 => (Instruction::ROR, ZeroPage, 5, 2),
		0x68 => (Instruction::PLA, Implicit, 4, 1),
		0x69 => (Instruction::ADC, Immediate, 2, 2),
		0x6a => (Instruction::ROR, Accumulator, 2, 1),
		0x6c => (Instruction::JMP, Indirect, 5, 3),
		0x6d => (Instruction::ADC, Absolute, 4, 3),
		0x6e => (Instruction::ROR, Absolute, 6, 3),
		0x70 => (Instruction::BVS, Relative, 2, 2), /*boundary*/
		0x71 => (Instruction::ADC, IndirectY, 5, 2), /*boundary*/
		0x75 => (Instruction::ADC, ZeroPageX, 4, 2),
		0x76 => (Instruction::ROR, ZeroPageX, 6, 2),
		0x78 => (Instruction::SEI, Implicit, 2, 1),
		0x79 => (Instruction::ADC, AbsoluteY, 4, 3), /*boundary*/
		0x7d => (Instruction::ADC, AbsoluteX, 4, 3), /*boundary*/
		0x7e => (Instruction::ROR, AbsoluteX, 7, 3),
		0x81 => (Instruction::STA, IndirectX, 6, 2),
		0x84 => (Instruction::STY, ZeroPage, 3, 2),
		0x85 => (Instruction::STA, ZeroPage, 3, 2),
		0x86 => (Instruction::STX, ZeroPage, 3, 2),
		0x88 => (Instruction::DEY, Implicit, 2, 1),
		0x8a => (Instruction::TXA, Implicit, 2, 1),
		0x8c => (Instruction::STY, Absolute, 4, 3),
		0x8d => (Instruction::STA, Absolute, 4, 3),
		0x8e => (Instruction::STX, Absolute, 4, 3),
		0x90 => (Instruction::BCC, Relative, 2, 2), /*boundary*/
		0x91 => (Instruction::STA, IndirectY, 6, 2),
		0x94 => (Instruction::STY, ZeroPageX, 4, 2),
		0x95 => (Instruction::STA, ZeroPageX, 4, 2),
		0x96 => (Instruction::STX, ZeroPageY, 4, 2),
		0x98 => (Instruction::TYA, Implicit, 2, 1),
		0x99 => (Instruction::STA, AbsoluteY, 5, 3),
		0x9a => (Instruction::TXS, Implicit, 2, 1),
		0x9d => (Instruction::STA, AbsoluteX, 5, 3),
		0xa0 => (Instruction::LDY, Immediate, 2, 2),
		0xa1 => (Instruction::LDA, IndirectX, 6, 2),
		0xa2 => (Instruction::LDX, Immediate, 2, 2),
		0xa4 => (Instruction::LDY, ZeroPage, 3, 2),
		0xa5 => (Instruction::LDA, ZeroPage, 3, 2),
		0xa6 => (Instruction::LDX, ZeroPage, 3, 2),
		0xa8 => (Instruction::TAY, Implicit, 2, 1),
		0xa9 => (Instruction::LDA, Immediate, 2, 2),
		0xaa => (Instruction::TAX, Implicit, 2, 1),
		0xac => (Instruction::LDY, Absolute, 4, 3),
		0xad => (Instruction::LDA, Absolute, 4, 3),
		0xae => (Instruction::LDX, Absolute, 4, 3),
		0xb0 => (Instruction::BCS, Relative, 3, 2), /*boundary*/
		0xb1 => (Instruction::LDA, IndirectY, 5, 2), /*boundary*/
		0xb4 => (Instruction::LDY, ZeroPageX, 4, 2),
		0xb5 => (Instruction::LDA, ZeroPageX, 4, 2),
		0xb6 => (Instruction::LDX, ZeroPageY, 4, 2),
		0xb9 => (Instruction::LDA, AbsoluteY, 4, 3), /*boundary*/
		0xba => (Instruction::TSX, Implicit, 2, 1),
		0xbc => (Instruction::LDY, AbsoluteX, 4, 3), /*boundary*/
		0xbd => (Instruction::LDA, AbsoluteX, 4, 3), /*boundary*/
		0xbe => (Instruction::LDX, AbsoluteY, 4, 3), /*boundary*/
		0xc0 => (Instruction::CPY, Immediate, 2, 2),
		0xc1 => (Instruction::CMP, IndirectX, 6, 2),
		0xc4 => (Instruction::CPY, ZeroPage, 3, 2),
		0xc5 => (Instruction::CMP, ZeroPage, 3, 2),
		0xc6 => (Instruction::DEC, ZeroPage, 5, 2),
		0xc8 => (Instruction::INY, Implicit, 2, 1),
		0xc9 => (Instruction::CMP, Immediate, 2, 2),
		0xca => (Instruction::DEX, Implicit, 2, 1),
		0xcc => (Instruction::CPY, Absolute, 4, 3),
		0xcd => (Instruction::CMP, Absolute, 4, 3),
		0xce => (Instruction::DEC, Absolute, 6, 3),
		0xd0 => (Instruction::BNE, Relative, 2, 2), /*boundary*/
		0xd1 => (Instruction::CMP, IndirectY, 5, 2), /*boundary*/
		0xd5 => (Instruction::CMP, ZeroPageX, 4, 2),
		0xd6 => (Instruction::DEC, ZeroPageX, 6, 2),
		0xd8 => (Instruction::CLD, Implicit, 2, 1),
		0xd9 => (Instruction::CMP, AbsoluteY, 4, 3), /*boundary*/
		0xdd => (Instruction::CMP, AbsoluteX, 4, 3), /*boundary*/
		0xde => (Instruction::DEC, AbsoluteX, 7, 3),
		0xe0 => (Instruction::CPX, Immediate, 2, 2),
		0xe1 => (Instruction::SBC, IndirectX, 6, 2),
		0xe4 => (Instruction::CPX, ZeroPage, 3, 2),
		0xe5 => (Instruction::SBC, ZeroPage, 3, 2),
		0xe6 => (Instruction::INC, ZeroPage, 5, 2),
		0xe8 => (Instruction::INX, Implicit, 2, 1),
		0xe9 => (Instruction::SBC, Immediate, 2, 2),
		0xea => (Instruction::NOP, Implicit, 2, 1),
		0xec => (Instruction::CPX, Absolute, 4, 3),
		0xed => (Instruction::SBC, Absolute, 4, 3),
		0xee => (Instruction::INC, Absolute, 6, 3),
		0xf0 => (Instruction::BEQ, Relative, 2, 2), /*boundary*/
		0xf1 => (Instruction::SBC, IndirectY, 5, 2),
		0xf5 => (Instruction::SBC, ZeroPageX, 4, 2),
		0xf6 => (Instruction::INC, ZeroPageX, 6, 2),
		0xf9 => (Instruction::SBC, AbsoluteY, 4, 3), /*boundary*/
		0xfd => (Instruction::SBC, AbsoluteX, 4, 3), /*boundary*/
		0xfe => (Instruction::INC, AbsoluteX, 7, 3),
		_ => handle_unknown_opcode(opcode)
	};

	RealizedInstruction {
    	instruction,
    	addr_mode,
    	cycles,
    	bytes
	}
}

fn handle_unknown_opcode(opcode: u8) -> (Instruction, AddressingMode, u16, u8) /* unused */ {
	panic!("Unknown opcode 0x{opcode:x}");
}

fn add_with_carry_and_update(state: &mut ProgramState, mem_val: u8, carry: u8) {
	let old_a = state.accumulator;

	let (result, carry) = add_with_carry_impl(old_a, mem_val, carry);

	state.accumulator = result;
	state.update_zero_neg_flags(result);
	state.update_flag(StatusFlag::Carry, carry);
	state.update_flag(StatusFlag::Overflow, (result ^ old_a) & (result ^ mem_val) & 0x80 != 0);
}

fn add_with_carry_impl(a:u8, b:u8, carry:u8) -> (u8, bool) {
	let first_add_result = a.overflowing_add(b);
	let second_add_result = first_add_result.0.overflowing_add(carry);

	(second_add_result.0, first_add_result.1 || second_add_result.1)
}