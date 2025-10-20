use crate::cpu::instruction::Instruction;
use crate::cpu::instruction::Instruction::*;
use crate::cpu::tests::test_mapper::TestMapper;
use crate::cpu::AddressingMode::*;
use crate::cpu::{instruction, AddressingMode, CoreMemory, CPU};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_instructions() {
    let cpu = &mut testing_cpu();

    /* ADC */
    /* simple addition test */
    ADC.apply(cpu, &Immediate, 0x05, 0x0);
    assert_eq!(cpu.accumulator, 5);
    /* now test with carry */
    cpu.status |= 1; /* set carry */
    ADC.apply(cpu, &Immediate, 0x10, 0x0);
    /* 0xfe + 0x5 + 1 = 0x4 */
    assert_eq!(cpu.accumulator, 0x16);
    /* test wrapping */
    ADC.apply(cpu, &Immediate, 0xf0, 0x0);
    /* carry was updated 0xf0 + 0x16 = 0x06*/
    assert_eq!(cpu.accumulator, 0x06);
    /* but that should've set carry flag on its own */
    ADC.apply(cpu, &Immediate, 0x0, 0x0);
    assert_eq!(cpu.accumulator, 0x07);
    cpu.write_mem(0x0010, 0x20);
    /* test that reading from memory doesn't lead to any issues */
    ADC.apply(cpu, &Absolute, 0x10, 0x0);
    assert_eq!(cpu.accumulator, 0x27);
    /* TODO check flags */

    /* AND */
    cpu.accumulator = 0b11110000;
    AND.apply(cpu, &Immediate, 0b10101010, 0x0);
    assert_eq!(cpu.accumulator, 0b10100000);
    /* TODO check flags */

    /* ASL */
    cpu.accumulator = 0b00111100;
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.accumulator, 0b01111000);
    /* TODO test carry flag */
}

#[test]
fn test_addressing_modes() {
    let cpu = &mut testing_cpu();

    cpu.write_mem(0x0010, 0x20);

    /* we can read memory through an addressing mode */
    assert_eq!(Absolute.deref(cpu, 0x10, 0x0), 0x20);
    /* we can write memory through an addressing mode */
    AddressingMode::Absolute.write(cpu, 0x11, 0x0, 0x80);
    assert_eq!(Absolute.deref(cpu, 0x11, 0x10), 0x80);

    /* we can read the accumulator through an addressing mode */
    cpu.accumulator = 0xaa;
    assert_eq!(Accumulator.deref(cpu, 0xab, 0xcd), 0xaa);

    /* testing the addresses themselves */

    /* Absolute */
    assert_eq!(Absolute.resolve_address(cpu, 0xab, 0xcd), 0xcdab);

    /* AbsoluteX */
    cpu.index_x = 0x05;
    assert_eq!(AbsoluteX.resolve_address(cpu, 0xab, 0xcd), 0xcdb0);

    /* AbsoluteY */
    cpu.index_y = 0x10;
    assert_eq!(AbsoluteY.resolve_address(cpu, 0xab, 0xcd), 0xcdbb);

    /* ZeroPage */
    assert_eq!(ZeroPage.resolve_address(cpu, 0xab, 0xcd), 0x00ab);

    /* ZeroPageX */
    cpu.index_x = 0x1;
    assert_eq!(ZeroPageX.resolve_address(cpu, 0xab, 0xcd), 0x00ac);
    /* wrapping case */
    cpu.index_x = 0xff;
    assert_eq!(ZeroPageX.resolve_address(cpu, 0xab, 0xcd), 0x00aa);

    /* ZeroPageY */
    cpu.index_y = 0x1;
    assert_eq!(ZeroPageY.resolve_address(cpu, 0xab, 0xcd), 0x00ac);
    /* wrapping case */
    cpu.index_y = 0xff;
    assert_eq!(ZeroPageY.resolve_address(cpu, 0xab, 0xcd), 0x00aa);
}

#[test]
fn test_opcodes() {
    /* ADC */
    test_opcode(0x69, ADC, Immediate, 2);
    test_opcode(0x65, ADC, ZeroPage, 3);
    test_opcode(0x75, ADC, ZeroPageX, 4);
    test_opcode(0x6d, ADC, Absolute, 4);
    test_opcode(0x7d, ADC, AbsoluteX, 4);
    test_opcode(0x79, ADC, AbsoluteY, 4);
    test_opcode(0x61, ADC, IndirectX, 6);
    test_opcode(0x71, ADC, IndirectY, 5);

    /* AND */
    test_opcode(0x29, AND, Immediate, 2);
    test_opcode(0x25, AND, ZeroPage, 3);
    test_opcode(0x35, AND, ZeroPageX, 4);
    test_opcode(0x2d, AND, Absolute, 4);
    test_opcode(0x3d, AND, AbsoluteX, 4);
    test_opcode(0x39, AND, AbsoluteY, 4);
    test_opcode(0x21, AND, IndirectX, 6);
    test_opcode(0x31, AND, IndirectY, 5);

    /* ASL */
    test_opcode(0x0a, ASL, Accumulator, 2);
    test_opcode(0x06, ASL, ZeroPage, 5);
    test_opcode(0x16, ASL, ZeroPageX, 6);
    test_opcode(0x0e, ASL, Absolute, 6);
    test_opcode(0x1e, ASL, AbsoluteX, 7);

    /* BCC */
    test_opcode(0x90, BCC, Relative, 2);

    /* BCS */
    test_opcode(0xb0, BCS, Relative, 2);

    /* BEQ */
    test_opcode(0xf0, BEQ, Relative, 2);

    /* BIT */
    test_opcode(0x24, BIT, ZeroPage, 3);
    test_opcode(0x2c, BIT, Absolute, 4);

    /* BMI */
    test_opcode(0x30, BMI, Relative, 2);

    /* BNE */
    test_opcode(0xd0, BNE, Relative, 2);

    /* BPL */
    test_opcode(0x10, BPL, Relative, 2);

    /* BRK */
    test_opcode(0x00, BRK, Implicit, 7);

    /* BVC */
    test_opcode(0x50, BVC, Relative, 2);

    /* BVS */
    test_opcode(0x70, BVS, Relative, 2);

    /* CLC */
    test_opcode(0x18, CLC, Implicit, 2);

    /* CLD */
    test_opcode(0xd8, CLD, Implicit, 2);

    /* CLI */
    test_opcode(0x58, CLI, Implicit, 2);

    /* CLV */
    test_opcode(0xb8, CLV, Implicit, 2);

    /* CMP */
    test_opcode(0xc9, CMP, Immediate, 2);
    test_opcode(0xc5, CMP, ZeroPage, 3);
    test_opcode(0xd5, CMP, ZeroPageX, 4);
    test_opcode(0xcd, CMP, Absolute, 4);
    test_opcode(0xdd, CMP, AbsoluteX, 4);
    test_opcode(0xd9, CMP, AbsoluteY, 4);
    test_opcode(0xc1, CMP, IndirectX, 6);
    test_opcode(0xd1, CMP, IndirectY, 5);

    /* CPX */
    test_opcode(0xe0, CPX, Immediate, 2);
    test_opcode(0xe4, CPX, ZeroPage, 3);
    test_opcode(0xec, CPX, Absolute, 4);

    /* CPY */
    test_opcode(0xc0, CPY, Immediate, 2);
    test_opcode(0xc4, CPY, ZeroPage, 3);
    test_opcode(0xcc, CPY, Absolute, 4);

    /* DEC */
    test_opcode(0xc6, DEC, ZeroPage, 5);
    test_opcode(0xd6, DEC, ZeroPageX, 6);
    test_opcode(0xce, DEC, Absolute, 6);
    test_opcode(0xde, DEC, AbsoluteX, 7);
}

fn test_opcode(
    opcode: u8,
    expected_instruction: Instruction,
    expected_addr_mode: AddressingMode,
    expected_cycles: u16,
) {
    let realized_instruction = instruction::from_opcode(opcode);
    assert_eq!(realized_instruction.instruction, expected_instruction);
    assert_eq!(realized_instruction.addr_mode, expected_addr_mode);
    assert_eq!(realized_instruction.cycles, expected_cycles);
}

fn testing_cpu() -> Box<CPU> {
    CPU::new(Box::new(CoreMemory::new_from_mapper(Rc::new(
        RefCell::new(Box::new(TestMapper::new())),
    ))))
}
