use crate::cpu::{AddressingMode, StatusFlag};

use crate::cpu::cpu::CPU;
use AddressingMode::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
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
    TYA, /* transfer value from Y into A; can set zero flag */

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
    CLI, /* Clear Interrupt Disable */
    CLV, /* Clear Overflow */
    SEC, /* Set Carry Flag */
    SED, /* Set Decimal Flag */
    SEI, /* Set Interrupt Disable */

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

impl Instruction {
    pub fn apply(&self, cpu: &mut CPU, addr_mode: &AddressingMode, b1: u8, b2: u8) -> u16 {
        let mut extra_cycles = 0;
        match self {
            Instruction::ADC => {
                let val = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                add_with_carry_and_update(cpu, val, StatusFlag::Carry.as_num(cpu));
            }
            Instruction::AND => {
                let mem_val = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.accumulator = cpu.accumulator & mem_val;
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::ASL => {
                let old_val: u8 = addr_mode.deref(cpu, b1, b2);
                let result = old_val << 1;
                cpu.update_flag(StatusFlag::Carry, old_val & 0x80 != 0);
                cpu.update_zero_neg_flags(result);
                addr_mode.write(cpu, b1, b2, result);
            }
            Instruction::BCC => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Carry, false, b1);
            }
            Instruction::BCS => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Carry, true, b1);
            }
            Instruction::BEQ => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Zero, true, b1);
            }
            Instruction::BIT => {
                let mem = addr_mode.deref(cpu, b1, b2);
                let val = cpu.accumulator & mem;
                cpu.update_flag(StatusFlag::Zero, val == 0);
                cpu.update_flag(StatusFlag::Overflow, mem & 0x40 != 0);
                cpu.update_flag(StatusFlag::Negative, mem & 0x80 != 0);
            }
            Instruction::BMI => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Negative, true, b1);
            }
            Instruction::BNE => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Zero, false, b1);
            }
            Instruction::BPL => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Negative, false, b1);
            }
            Instruction::BRK => {
                cpu.irq_with_offset(2);
            }
            Instruction::BVC => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Overflow, false, b1);
            }
            Instruction::BVS => {
                extra_cycles += Self::branch_instr(cpu, StatusFlag::Overflow, true, b1);
            }
            Instruction::CLC => {
                cpu.update_flag(StatusFlag::Carry, false);
            }
            Instruction::CLD => {
                cpu.update_flag(StatusFlag::Decimal, false);
            }
            Instruction::CLI => {
                /* TODO: The effect is delayed "one instruction".
                 * Does that mean one cycle, or until the next instruction?
                 * how to implement this?
                 */
                cpu.update_flag(StatusFlag::InterruptDisable, false);
            }
            Instruction::CLV => {
                cpu.update_flag(StatusFlag::Overflow, false);
            }
            Instruction::CMP => {
                Self::compare(cpu, addr_mode, b1, b2, cpu.accumulator, &mut extra_cycles);
            }
            Instruction::CPX => {
                Self::compare(cpu, addr_mode, b1, b2, cpu.index_x, &mut extra_cycles);
            }
            Instruction::CPY => {
                Self::compare(cpu, addr_mode, b1, b2, cpu.index_y, &mut extra_cycles);
            }
            Instruction::DEC => {
                let new_val = addr_mode.deref(cpu, b1, b2).wrapping_sub(1);
                addr_mode.write(cpu, b1, b2, new_val);
                cpu.update_zero_neg_flags(new_val);
            }
            Instruction::DEX => {
                cpu.index_x = cpu.index_x.wrapping_sub(1);
                cpu.update_zero_neg_flags(cpu.index_x);
            }
            Instruction::DEY => {
                cpu.index_y = cpu.index_y.wrapping_sub(1);
                cpu.update_zero_neg_flags(cpu.index_y);
            }
            Instruction::EOR => {
                let mem_val = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.accumulator = cpu.accumulator ^ mem_val;
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::INC => {
                let new_val = addr_mode.deref(cpu, b1, b2).wrapping_add(1);
                addr_mode.write(cpu, b1, b2, new_val);
                cpu.update_zero_neg_flags(new_val);
            }
            Instruction::INX => {
                cpu.index_x = cpu.index_x.wrapping_add(1);
                cpu.update_zero_neg_flags(cpu.index_x);
            }
            Instruction::INY => {
                cpu.index_y = cpu.index_y.wrapping_add(1);
                cpu.update_zero_neg_flags(cpu.index_y);
            }
            Instruction::JMP => {
                cpu.program_counter = addr_mode.resolve_address(cpu, b1, b2);
            }
            Instruction::JSR => {
                cpu.push_memory_loc(cpu.program_counter + 2);
                cpu.program_counter = addr_mode.resolve_address(cpu, b1, b2);
            }
            Instruction::LDA => {
                cpu.accumulator =
                    addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::LDX => {
                cpu.index_x = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.update_zero_neg_flags(cpu.index_x);
            }
            Instruction::LDY => {
                cpu.index_y = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.update_zero_neg_flags(cpu.index_y);
            }
            Instruction::LSR => {
                let val = addr_mode.deref(cpu, b1, b2);
                let new_val = val >> 1;
                addr_mode.write(cpu, b1, b2, new_val);
                cpu.update_flag(StatusFlag::Carry, (val & 0x1) != 0);
                cpu.update_flag(StatusFlag::Zero, new_val == 0);
                cpu.update_flag(StatusFlag::Negative, false);
            }
            Instruction::NOP => { /* nothing */ }
            Instruction::ORA => {
                cpu.accumulator |=
                    addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::PHA => {
                cpu.push(cpu.accumulator);
            }
            Instruction::PHP => {
                /* pushes status onto the stack, with the 'B' flag (bit 4) on */
                cpu.push(cpu.status | (1 << 4));
            }
            Instruction::PLA => {
                cpu.accumulator = cpu.pop();
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::PLP => {
                /* reads status from the stack, except for bits 4 and 5 */
                let val = cpu.pop();
                cpu.status = (cpu.status & 0x30) | (val & !0x30);
                /* TODO: update to interrupt disable should be delayed one instruction */
            }
            Instruction::ROL => {
                let val = addr_mode.deref(cpu, b1, b2);
                let mut result = StatusFlag::Carry.as_num(cpu);
                result = result | (val << 1);
                addr_mode.write(cpu, b1, b2, result);
                cpu.update_flag(StatusFlag::Carry, val & 0x80 != 0);
                cpu.update_zero_neg_flags(result);
            }
            Instruction::ROR => {
                let val = addr_mode.deref(cpu, b1, b2);
                let mut result = StatusFlag::Carry.as_num(cpu) << 7;
                result = result | (val >> 1);
                addr_mode.write(cpu, b1, b2, result);
                cpu.update_flag(StatusFlag::Carry, val & 0x1 != 0);
                cpu.update_zero_neg_flags(result);
            }
            Instruction::RTI => {
                /* TODO: this works as is, but should it be loading flags 4 and 5? */
                cpu.status = cpu.pop();

                cpu.program_counter = cpu.pop_memory_loc();
            }
            Instruction::RTS => {
                cpu.program_counter = cpu.pop_memory_loc() + 1;
            }
            Instruction::SBC => {
                let val = addr_mode.deref_check_boundary_cross(cpu, b1, b2, &mut extra_cycles);
                add_with_carry_and_update(cpu, !val, StatusFlag::Carry.as_num(cpu));
            }
            Instruction::SEC => {
                StatusFlag::Carry.update_bool(cpu, true);
            }
            Instruction::SED => {
                StatusFlag::Decimal.update_bool(cpu, true);
            }
            Instruction::SEI => {
                /* TODO: The effect is delayed "one instruction".
                 * Does that mean one cycle, or until the next instruction?
                 * how to implement this?
                 */
                cpu.update_flag(StatusFlag::InterruptDisable, true);
            }
            Instruction::STA => {
                addr_mode.write(cpu, b1, b2, cpu.accumulator);
            }
            Instruction::STX => {
                addr_mode.write(cpu, b1, b2, cpu.index_x);
            }
            Instruction::STY => {
                addr_mode.write(cpu, b1, b2, cpu.index_y);
            }
            Instruction::TAX => {
                cpu.index_x = cpu.accumulator;
                cpu.update_zero_neg_flags(cpu.index_x);
            }
            Instruction::TAY => {
                cpu.index_y = cpu.accumulator;
                cpu.update_zero_neg_flags(cpu.index_y);
            }
            Instruction::TSX => {
                cpu.index_x = cpu.s_register;
                cpu.update_zero_neg_flags(cpu.index_x);
            }
            Instruction::TXA => {
                cpu.accumulator = cpu.index_x;
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
            Instruction::TXS => {
                cpu.s_register = cpu.index_x;
                /* doesn't update flags! multiple sources agree on this ? */
            }
            Instruction::TYA => {
                cpu.accumulator = cpu.index_y;
                cpu.update_zero_neg_flags(cpu.accumulator);
            }
        }

        extra_cycles
    }

    fn branch_instr(cpu: &mut CPU, flag: StatusFlag, is_positive: bool, offset: u8) -> u16 {
        if is_positive == flag.is_set(cpu) {
            let old_pc = cpu.program_counter;
            cpu.program_counter = Relative.resolve_address(cpu, offset, 0);
            /* TODO comment */
            if old_pc & !0xff != cpu.program_counter & !0xff {
                2
            } else {
                1
            }
        } else {
            0
        }
    }

    fn compare(
        cpu: &mut CPU,
        addr_mode: &AddressingMode,
        b1: u8,
        b2: u8,
        compare_val: u8,
        extra_cycles: &mut u16,
    ) {
        let mem_val = addr_mode.deref_check_boundary_cross(cpu, b1, b2, extra_cycles);

        cpu.update_flag(StatusFlag::Carry, compare_val >= mem_val);
        cpu.update_flag(StatusFlag::Zero, compare_val == mem_val);
        cpu.update_flag(
            StatusFlag::Negative,
            compare_val.wrapping_sub(mem_val) & 0x80 != 0,
        );
    }
}

#[derive(Debug)]
pub struct RealizedInstruction {
    pub instruction: Instruction,
    pub addr_mode: AddressingMode,
    pub cycles: u16,
}

impl RealizedInstruction {
    pub fn apply(&self, cpu: &mut CPU, b1: u8, b2: u8) -> u16 {
        let extra_cycles = self.instruction.apply(cpu, &self.addr_mode, b1, b2);
        /* note that this holds even for branching instructions (but not jump instructions): program counter needs to be
         * incremented by the number of bytes for instruction, arguments
         */

        match &self.instruction {
            Instruction::JMP => {}
            Instruction::JSR => {}
            Instruction::RTS => {}
            Instruction::RTI => {}
            Instruction::BRK => {} // acts like a JMP
            _ => {
                cpu.program_counter = cpu
                    .program_counter
                    .wrapping_add(self.addr_mode.get_bytes() as u16);
            }
        }

        extra_cycles
    }
}

pub fn from_opcode(opcode: u8) -> RealizedInstruction {
    let (instruction, addr_mode, cycles) = match opcode {
        /* TODO: instructions marked 'boundary' take longer if crossing
         * a page boundary */
        /* branch instructions also take an extra cycle if branch taken */
        0x00 => (Instruction::BRK, Implicit, 7),
        0x01 => (Instruction::ORA, IndirectX, 6),
        0x05 => (Instruction::ORA, ZeroPage, 3),
        0x06 => (Instruction::ASL, ZeroPage, 5),
        0x08 => (Instruction::PHP, Implicit, 3),
        0x09 => (Instruction::ORA, Immediate, 2),
        0x0a => (Instruction::ASL, Accumulator, 2),
        0x0d => (Instruction::ORA, Absolute, 4),
        0x0e => (Instruction::ASL, Absolute, 6),
        0x10 => (Instruction::BPL, Relative, 2), /*boundary*/
        0x11 => (Instruction::ORA, IndirectY, 5), /*boundary*/
        0x15 => (Instruction::ORA, ZeroPageX, 4),
        0x16 => (Instruction::ASL, ZeroPageX, 6),
        0x18 => (Instruction::CLC, Implicit, 2),
        0x19 => (Instruction::ORA, AbsoluteY, 4), /*boundary*/
        0x1a => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0x1d => (Instruction::ORA, AbsoluteX, 4), /*boundary*/
        0x1e => (Instruction::ASL, AbsoluteX, 7),
        0x20 => (Instruction::JSR, Absolute, 6),
        0x21 => (Instruction::AND, IndirectX, 6),
        0x24 => (Instruction::BIT, ZeroPage, 3),
        0x25 => (Instruction::AND, ZeroPage, 3),
        0x26 => (Instruction::ROL, ZeroPage, 5),
        0x28 => (Instruction::PLP, Implicit, 4),
        0x29 => (Instruction::AND, Immediate, 2),
        0x2a => (Instruction::ROL, Accumulator, 2),
        0x2c => (Instruction::BIT, Absolute, 4),
        0x2d => (Instruction::AND, Absolute, 4),
        0x2e => (Instruction::ROL, Absolute, 6),
        0x30 => (Instruction::BMI, Relative, 2), /*boundary*/
        0x31 => (Instruction::AND, IndirectY, 5), /*boundary*/
        0x35 => (Instruction::AND, ZeroPageX, 4),
        0x36 => (Instruction::ROL, ZeroPageX, 6),
        0x38 => (Instruction::SEC, Implicit, 2),
        0x39 => (Instruction::AND, AbsoluteY, 4), /*boundary*/
        0x3a => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0x3d => (Instruction::AND, AbsoluteX, 4), /*boundary*/
        0x3e => (Instruction::ROL, AbsoluteX, 7),
        0x40 => (Instruction::RTI, Implicit, 6),
        0x41 => (Instruction::EOR, IndirectX, 6),
        0x45 => (Instruction::EOR, ZeroPage, 3),
        0x46 => (Instruction::LSR, ZeroPage, 5),
        0x48 => (Instruction::PHA, Implicit, 3),
        0x49 => (Instruction::EOR, Immediate, 2),
        0x4a => (Instruction::LSR, Accumulator, 2),
        0x4c => (Instruction::JMP, Absolute, 3),
        0x4d => (Instruction::EOR, Absolute, 4),
        0x4e => (Instruction::LSR, Absolute, 6),
        0x50 => (Instruction::BVC, Relative, 2), /*boundary*/
        0x51 => (Instruction::EOR, IndirectY, 5), /*boundary*/
        0x55 => (Instruction::EOR, ZeroPageX, 4),
        0x56 => (Instruction::LSR, ZeroPageX, 6),
        0x58 => (Instruction::CLI, Implicit, 2),
        0x59 => (Instruction::EOR, AbsoluteY, 4), /*boundary*/
        0x5a => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0x5d => (Instruction::EOR, AbsoluteX, 4), /*boundary*/
        0x5e => (Instruction::LSR, AbsoluteX, 7),
        0x60 => (Instruction::RTS, Implicit, 6),
        0x61 => (Instruction::ADC, IndirectX, 6),
        0x65 => (Instruction::ADC, ZeroPage, 3),
        0x66 => (Instruction::ROR, ZeroPage, 5),
        0x68 => (Instruction::PLA, Implicit, 4),
        0x69 => (Instruction::ADC, Immediate, 2),
        0x6a => (Instruction::ROR, Accumulator, 2),
        0x6c => (Instruction::JMP, Indirect, 5),
        0x6d => (Instruction::ADC, Absolute, 4),
        0x6e => (Instruction::ROR, Absolute, 6),
        0x70 => (Instruction::BVS, Relative, 2), /*boundary*/
        0x71 => (Instruction::ADC, IndirectY, 5), /*boundary*/
        0x75 => (Instruction::ADC, ZeroPageX, 4),
        0x76 => (Instruction::ROR, ZeroPageX, 6),
        0x78 => (Instruction::SEI, Implicit, 2),
        0x79 => (Instruction::ADC, AbsoluteY, 4), /*boundary*/
        0x7a => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0x7d => (Instruction::ADC, AbsoluteX, 4), /*boundary*/
        0x7e => (Instruction::ROR, AbsoluteX, 7),
        0x81 => (Instruction::STA, IndirectX, 6),
        0x84 => (Instruction::STY, ZeroPage, 3),
        0x85 => (Instruction::STA, ZeroPage, 3),
        0x86 => (Instruction::STX, ZeroPage, 3),
        0x88 => (Instruction::DEY, Implicit, 2),
        0x8a => (Instruction::TXA, Implicit, 2),
        0x8c => (Instruction::STY, Absolute, 4),
        0x8d => (Instruction::STA, Absolute, 4),
        0x8e => (Instruction::STX, Absolute, 4),
        0x90 => (Instruction::BCC, Relative, 2), /*boundary*/
        0x91 => (Instruction::STA, IndirectY, 6),
        0x94 => (Instruction::STY, ZeroPageX, 4),
        0x95 => (Instruction::STA, ZeroPageX, 4),
        0x96 => (Instruction::STX, ZeroPageY, 4),
        0x98 => (Instruction::TYA, Implicit, 2),
        0x99 => (Instruction::STA, AbsoluteY, 5),
        0x9a => (Instruction::TXS, Implicit, 2),
        0x9d => (Instruction::STA, AbsoluteX, 5),
        0xa0 => (Instruction::LDY, Immediate, 2),
        0xa1 => (Instruction::LDA, IndirectX, 6),
        0xa2 => (Instruction::LDX, Immediate, 2),
        0xa4 => (Instruction::LDY, ZeroPage, 3),
        0xa5 => (Instruction::LDA, ZeroPage, 3),
        0xa6 => (Instruction::LDX, ZeroPage, 3),
        0xa8 => (Instruction::TAY, Implicit, 2),
        0xa9 => (Instruction::LDA, Immediate, 2),
        0xaa => (Instruction::TAX, Implicit, 2),
        0xac => (Instruction::LDY, Absolute, 4),
        0xad => (Instruction::LDA, Absolute, 4),
        0xae => (Instruction::LDX, Absolute, 4),
        0xb0 => (Instruction::BCS, Relative, 2), /*boundary*/
        0xb1 => (Instruction::LDA, IndirectY, 5), /*boundary*/
        0xb4 => (Instruction::LDY, ZeroPageX, 4),
        0xb5 => (Instruction::LDA, ZeroPageX, 4),
        0xb6 => (Instruction::LDX, ZeroPageY, 4),
        0xb8 => (Instruction::CLV, Implicit, 2),
        0xb9 => (Instruction::LDA, AbsoluteY, 4), /*boundary*/
        0xba => (Instruction::TSX, Implicit, 2),
        0xbc => (Instruction::LDY, AbsoluteX, 4), /*boundary*/
        0xbd => (Instruction::LDA, AbsoluteX, 4), /*boundary*/
        0xbe => (Instruction::LDX, AbsoluteY, 4), /*boundary*/
        0xc0 => (Instruction::CPY, Immediate, 2),
        0xc1 => (Instruction::CMP, IndirectX, 6),
        0xc4 => (Instruction::CPY, ZeroPage, 3),
        0xc5 => (Instruction::CMP, ZeroPage, 3),
        0xc6 => (Instruction::DEC, ZeroPage, 5),
        0xc8 => (Instruction::INY, Implicit, 2),
        0xc9 => (Instruction::CMP, Immediate, 2),
        0xca => (Instruction::DEX, Implicit, 2),
        0xcc => (Instruction::CPY, Absolute, 4),
        0xcd => (Instruction::CMP, Absolute, 4),
        0xce => (Instruction::DEC, Absolute, 6),
        0xd0 => (Instruction::BNE, Relative, 2), /*boundary*/
        0xd1 => (Instruction::CMP, IndirectY, 5), /*boundary*/
        0xd5 => (Instruction::CMP, ZeroPageX, 4),
        0xd6 => (Instruction::DEC, ZeroPageX, 6),
        0xd8 => (Instruction::CLD, Implicit, 2),
        0xd9 => (Instruction::CMP, AbsoluteY, 4), /*boundary*/
        0xda => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0xdd => (Instruction::CMP, AbsoluteX, 4), /*boundary*/
        0xde => (Instruction::DEC, AbsoluteX, 7),
        0xe0 => (Instruction::CPX, Immediate, 2),
        0xe1 => (Instruction::SBC, IndirectX, 6),
        0xe4 => (Instruction::CPX, ZeroPage, 3),
        0xe5 => (Instruction::SBC, ZeroPage, 3),
        0xe6 => (Instruction::INC, ZeroPage, 5),
        0xe8 => (Instruction::INX, Implicit, 2),
        0xe9 => (Instruction::SBC, Immediate, 2),
        0xea => (Instruction::NOP, Implicit, 2),
        0xec => (Instruction::CPX, Absolute, 4),
        0xed => (Instruction::SBC, Absolute, 4),
        0xee => (Instruction::INC, Absolute, 6),
        0xf0 => (Instruction::BEQ, Relative, 2), /*boundary*/
        0xf1 => (Instruction::SBC, IndirectY, 5),
        0xf5 => (Instruction::SBC, ZeroPageX, 4),
        0xf6 => (Instruction::INC, ZeroPageX, 6),
        0xf8 => (Instruction::SED, Implicit, 2),
        0xf9 => (Instruction::SBC, AbsoluteY, 4), /*boundary*/
        0xfa => (Instruction::NOP, Implicit, 2),  /* unofficial */
        0xfd => (Instruction::SBC, AbsoluteX, 4), /*boundary*/
        0xfe => (Instruction::INC, AbsoluteX, 7),
        _ => handle_unknown_opcode(opcode),
    };

    RealizedInstruction {
        instruction,
        addr_mode,
        cycles,
    }
}

fn handle_unknown_opcode(opcode: u8) -> (Instruction, AddressingMode, u16) /* unused */ {
    panic!("Unknown opcode 0x{opcode:x}");
}

fn add_with_carry_and_update(cpu: &mut CPU, mem_val: u8, carry: u8) {
    let old_a = cpu.accumulator;

    let (result, carry) = add_with_carry_impl(old_a, mem_val, carry);

    cpu.accumulator = result;
    cpu.update_zero_neg_flags(result);
    cpu.update_flag(StatusFlag::Carry, carry);
    cpu.update_flag(
        StatusFlag::Overflow,
        (result ^ old_a) & (result ^ mem_val) & 0x80 != 0,
    );
}

fn add_with_carry_impl(a: u8, b: u8, carry: u8) -> (u8, bool) {
    let first_add_result = a.overflowing_add(b);
    let second_add_result = first_add_result.0.overflowing_add(carry);

    (
        second_add_result.0,
        first_add_result.1 || second_add_result.1,
    )
}
