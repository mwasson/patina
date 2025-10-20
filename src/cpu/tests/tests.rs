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
    /* 0x0 + 0x5 = 0x5 */
    /* TODO clean up all these gross flag accesses */
    assert_eq!(cpu.accumulator, 5);
    assert_eq!(cpu.status & 1 != 0, false); // no carry
    assert_eq!(cpu.status & 2 != 0, false); // not zero
    assert_eq!(cpu.status & (1 << 6) != 0, false); // no overflow
    assert_eq!(cpu.status & (1 << 7) != 0, false); // positive result
    /* now test with carry */
    cpu.status |= 1; /* set carry */
    ADC.apply(cpu, &Immediate, 0x10, 0x0);
    /* 0x5 + 0x10 + 1 = 0x16 */
    /* TODO clean up all these gross flag accesses */

    assert_eq!(cpu.accumulator, 0x16);
    assert_eq!(cpu.status & 1 != 0, false); // no carry
    assert_eq!(cpu.status & 2 != 0, false); // not zero
    assert_eq!(cpu.status & (1 << 6) != 0, false); // no overflow
    assert_eq!(cpu.status & (1 << 7) != 0, false); // positive result
    /* test wrapping */
    ADC.apply(cpu, &Immediate, 0xf0, 0x0);
    /* carry was updated, so: 0x16 + 0xf0 = 0x06*/
    /* TODO clean up all these gross flag accesses */
    assert_eq!(cpu.accumulator, 0x06);
    assert_eq!(cpu.status & 1 != 0, true); // carry
    assert_eq!(cpu.status & 2 != 0, false); // not zero
    assert_eq!(cpu.status & (1 << 6) != 0, false); // not overflow
    assert_eq!(cpu.status & (1 << 7) != 0, false); // positive result
    /* should've set carry flag on its own */
    /* 0x06 + 0x00 + 1 = 0x07 */
    ADC.apply(cpu, &Immediate, 0x0, 0x0);
    /* TODO clean up all these gross flag accesses */

    assert_eq!(cpu.accumulator, 0x07);
    assert_eq!(cpu.status & 1 != 0, false); // no carry
    assert_eq!(cpu.status & 2 != 0, false); // not zero
    assert_eq!(cpu.status & (1 << 6) != 0, false); // no overflow
    assert_eq!(cpu.status & (1 << 7) != 0, false); // positive result
    cpu.write_mem(0x0010, 0x20);
    /* test that reading from memory doesn't lead to any issues */
    ADC.apply(cpu, &Absolute, 0x10, 0x0);
    assert_eq!(cpu.accumulator, 0x27);
    /* TODO clean up all these gross flag accesses */
    /* test underflow, zero */
    cpu.accumulator = 0x80;
    ADC.apply(cpu, &Immediate, 0x80, 0x0);
    assert_eq!(cpu.accumulator, 0x0);
    assert_eq!(cpu.status & 1 != 0, true); // carry (unsigned overflow)
    assert_eq!(cpu.status & 2 != 0, true); // is zero
    assert_eq!(cpu.status & (1 << 6) != 0, true); // underflow
    assert_eq!(cpu.status & (1 << 7) != 0, false); // zero result
    /* finally test negative, from overflow */
    cpu.accumulator = 0x40;
    ADC.apply(cpu, &Immediate, 0x40, 0x0);
    assert_eq!(cpu.accumulator, 0x81); /* note carry bit was set */
    assert_eq!(cpu.status & 1 != 0, false); // no carry (unsigned overflow)
    assert_eq!(cpu.status & 2 != 0, false); // is zero
    assert_eq!(cpu.status & (1 << 6) != 0, true); // signed overflow
    assert_eq!(cpu.status & (1 << 7) != 0, true); // negative result


    /* AND */
    cpu.accumulator = 0b11110000;
    AND.apply(cpu, &Immediate, 0b10101010, 0x0);
    assert_eq!(cpu.accumulator, 0b10100000);
    /* TODO clean up flag access */
    assert_eq!(cpu.status & (1 << 7) != 0, true); /* result is negative */
    assert_eq!(cpu.status & (1 << 2) != 0, false); /* result is not zero */
    AND.apply(cpu, &Immediate, 0b00001111, 0x0);
    assert_eq!(cpu.status & (1 << 7) != 0, false); /* result is not negative */
    assert_eq!(cpu.status & (1 << 1) != 0, true); /* result is zero */

    /* ASL */
    cpu.accumulator = 0b00111100;
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.accumulator, 0b01111000);
    /* TODO clean up flags */
    assert_eq!(cpu.status & 1 != 0, false); // no carry
    assert_eq!(cpu.status & 2 != 0, false); // result is not zero
    assert_eq!(cpu.status & 7 != 0, false); // result not negative
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.accumulator, 0b11110000);
    assert_eq!(cpu.status & 1 != 0, false); // no carry
    assert_eq!(cpu.status & 2 != 0, false); // result is not zero
    assert_eq!(cpu.status & (1 << 7) != 0, true); // result is negative
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.accumulator, 0b11100000);
    assert_eq!(cpu.status & 1 != 0, true); // carry
    assert_eq!(cpu.status & 2 != 0, false); // result is not zero
    assert_eq!(cpu.status & (1 << 7) != 0, true); // result is negative
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.status & 2 != 0, false); // result is STILL not zero
    ASL.apply(cpu, &Accumulator, 0xab, 0xcd);
    assert_eq!(cpu.status & 2 != 0, true); // result *is* zero

    /* BCC */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 1; /* carry set */
    let extra_cycles = BCC.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf000); /* no branch */
    assert_eq!(extra_cycles, 0);
    /* TODO clean up use of flags */
    cpu.status = 0; /* carry clear */
    let extra_cycles = BCC.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    /* branch; relative to PC+2, but note that the CPU handles the +2, not the instruction */
    /* TODO test that */
    assert_eq!(cpu.program_counter, 0xf020);
    assert_eq!(extra_cycles, 1);
    /* checking for extra cycles when branching and it leads to page cross -
     * in this case 0x80 = -128, so this also tests negative offsets */
    cpu.program_counter = 0xf000;
    let extra_cycles = BCC.apply(cpu, &Relative, 0x80, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xef80);
    assert_eq!(extra_cycles, 2);

    /* BCS */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 1; /* carry set */
    let extra_cycles = BCS.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 0; /* carry clear */
    let extra_cycles = BCS.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BEQ */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 2; /* zero set */
    let extra_cycles = BEQ.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 0; /* zero clear */
    let extra_cycles = BEQ.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BIT */
    cpu.status = 0;
    cpu.accumulator = 0b11111111;
    cpu.write_mem(0x0010, 0b11111111);
    BIT.apply(cpu, &Absolute, 0x10, 0x00);
    assert_eq!(cpu.accumulator, 0b11111111); /* doesn't do anything to the accumulator! */
    /* TODO clean up flag access */
    assert_eq!(cpu.status & 2 != 0, false); // result is non-zero
    assert_eq!(cpu.status & (1 << 6) != 0, true); // input had bit 6 set
    assert_eq!(cpu.status & (1 << 7) != 0, true); // input had bit 7 set
    cpu.accumulator = 0b11000000;
    cpu.write_mem(0x0010, 0b00111111);
    BIT.apply(cpu, &Absolute, 0x10, 0x00);
    assert_eq!(cpu.accumulator, 0b11000000); /* no effect on accumulator */
    /* TODO clean up flag access */
    assert_eq!(cpu.status & 2 != 0, true); // result is zero
    assert_eq!(cpu.status & (1 << 6) != 0, false); // input did not have bit 6 set
    assert_eq!(cpu.status & (1 << 7) != 0, false); // input did not have bit 7 set

    /* BMI */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 1 << 7; /* negative set */
    let extra_cycles = BMI.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 0; /* negative clear */
    let extra_cycles = BMI.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BNE */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 0; /* zero clear */
    let extra_cycles = BNE.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 2; /* zero set */
    let extra_cycles = BNE.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BPL */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 0; /* negative clear */
    let extra_cycles = BPL.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 1 << 7; /* negative set */
    let extra_cycles = BPL.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BRK */
    cpu.s_register = 0xf0;
    cpu.status = 0b11000000;
    cpu.program_counter = 0x1234;
    cpu.write_mem(0xfffe, 0xbc); /* IRQ handler */
    cpu.write_mem(0xffff, 0x8a); /* IRQ handler */
    BRK.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0x8abc);
    assert_eq!(cpu.s_register, 0xf0 - 3);
    assert_eq!(cpu.read_mem(0x01f0), 0x12); /* old PC upper byte */
    assert_eq!(cpu.read_mem(0x01f0 - 1), 0x36); /* old PC lower byte + 2 */
    assert_eq!(cpu.read_mem(0x01f0 - 2), 0b11110000); /* B set but not I */
    assert_eq!(cpu.status, 0b11000100); /* I set but not B */

    /* BVC */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 0; /* overflow clear */
    let extra_cycles = BVC.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 1 << 6; /* overflow set */
    let extra_cycles = BVC.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* BVS */
    cpu.program_counter = 0xf000;
    /* TODO clean up use of flags */
    cpu.status = 1 << 6; /* overflow set */
    let extra_cycles = BVS.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* branches */
    assert_eq!(extra_cycles, 1);
    cpu.status = 0; /* overflow clear */
    let extra_cycles = BVS.apply(cpu, &Relative, 0x20, 0xcd /* unused */);
    assert_eq!(cpu.program_counter, 0xf020); /* didn't branch */
    assert_eq!(extra_cycles, 0);

    /* CLC */
    /* TODO clean up use of flags */
    cpu.status = 0b11111110;
    CLC.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0b11111110); /* no effect if carry already clear */
    cpu.status = 1;
    CLC.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0); /* clears carry */

    /* CLD */
    /* TODO clean up use of flags */
    cpu.status = 0b1111_0111;
    CLD.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0b1111_0111); /* no effect if decimal already clear */
    cpu.status = 0b0000_1000;
    CLD.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0); /* clears decimal */

    /* CLI */
    /* TODO clean up use of flags */
    cpu.status = 0b1111_1011;
    CLI.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0b1111_1011); /* no effect if interrupt disable already clear */
    cpu.status = 0b0000_0100;
    CLI.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0); /* clears interrupt disable */

    /* CLV */
    /* TODO clean up use of flags */
    cpu.status = 0b1011_1111;
    CLV.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0b1011_1111); /* no effect if overflow already clear */
    cpu.status = 0b0100_0000;
    CLV.apply(cpu, &Implicit, 0xab /* unused */, 0xcd /* unused */);
    assert_eq!(cpu.status, 0); /* clears overflow */
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

    /* DEX */
    test_opcode(0xca, DEX, Implicit, 2);

    /* DEY */
    test_opcode(0x88, DEY, Implicit, 2);

    /* EOR */
    test_opcode(0x49, EOR, Immediate, 2);
    test_opcode(0x45, EOR, ZeroPage, 3);
    test_opcode(0x55, EOR, ZeroPageX, 4);
    test_opcode(0x4d, EOR, Absolute, 4);
    test_opcode(0x5d, EOR, AbsoluteX, 4);
    test_opcode(0x59, EOR, AbsoluteY, 4);
    test_opcode(0x41, EOR, IndirectX, 6);
    test_opcode(0x51, EOR, IndirectY, 5);

    /* INC */
    test_opcode(0xe6, INC, ZeroPage, 5);
    test_opcode(0xf6, INC, ZeroPageX, 6);
    test_opcode(0xee, INC, Absolute, 6);
    test_opcode(0xfe, INC, AbsoluteX, 7);

    /* INX */
    test_opcode(0xe8, INX, Implicit, 2);

    /* INY */
    test_opcode(0xc8, INY, Implicit, 2);

    /* JMP */
    test_opcode(0x4c, JMP, Absolute, 3);
    test_opcode(0x6c, JMP, Indirect, 5);

    /* JSR */
    test_opcode(0x20, JSR, Absolute, 6);

    /* LDA */
    test_opcode(0xa9, LDA, Immediate, 2);
    test_opcode(0xa5, LDA, ZeroPage, 3);
    test_opcode(0xb5, LDA, ZeroPageX, 4);
    test_opcode(0xad, LDA, Absolute, 4);
    test_opcode(0xbd, LDA, AbsoluteX, 4);
    test_opcode(0xb9, LDA, AbsoluteY, 4);
    test_opcode(0xa1, LDA, IndirectX, 6);
    test_opcode(0xb1, LDA, IndirectY, 5);

    /* LDX */
    test_opcode(0xa2, LDX, Immediate, 2);
    test_opcode(0xa6, LDX, ZeroPage, 3);
    test_opcode(0xb6, LDX, ZeroPageY, 4);
    test_opcode(0xae, LDX, Absolute, 4);
    test_opcode(0xbe, LDX, AbsoluteY, 4);

    /* LDY */
    test_opcode(0xa0, LDY, Immediate, 2);
    test_opcode(0xa4, LDY, ZeroPage, 3);
    test_opcode(0xb4, LDY, ZeroPageX, 4);
    test_opcode(0xac, LDY, Absolute, 4);
    test_opcode(0xbc, LDY, AbsoluteX, 4);

    /* LSR */
    test_opcode(0x4a, LSR, Accumulator, 2);
    test_opcode(0x46, LSR, ZeroPage, 5);
    test_opcode(0x56, LSR, ZeroPageX, 6);
    test_opcode(0x4e, LSR, Absolute, 6);
    test_opcode(0x5e, LSR, AbsoluteX, 7);

    /* NOP */
    test_opcode(0xea, NOP, Implicit, 2);

    /* ORA */
    test_opcode(0x09, ORA, Immediate, 2);
    test_opcode(0x05, ORA, ZeroPage, 3);
    test_opcode(0x15, ORA, ZeroPageX, 4);
    test_opcode(0x0d, ORA, Absolute, 4);
    test_opcode(0x1d, ORA, AbsoluteX, 4);
    test_opcode(0x19, ORA, AbsoluteY, 4);
    test_opcode(0x01, ORA, IndirectX, 6);
    test_opcode(0x11, ORA, IndirectY, 5);

    /* PHA */
    test_opcode(0x48, PHA, Implicit, 3);

    /* PHP */
    test_opcode(0x08, PHP, Implicit, 3);

    /* PLA */
    test_opcode(0x68, PLA, Implicit, 4);

    /* PLP */
    test_opcode(0x28, PLP, Implicit, 4);

    /* ROL */
    test_opcode(0x2a, ROL, Accumulator, 2);
    test_opcode(0x26, ROL, ZeroPage, 5);
    test_opcode(0x36, ROL, ZeroPageX, 6);
    test_opcode(0x2e, ROL, Absolute, 6);
    test_opcode(0x3e, ROL, AbsoluteX, 7);

    /* ROL */
    test_opcode(0x6a, ROR, Accumulator, 2);
    test_opcode(0x66, ROR, ZeroPage, 5);
    test_opcode(0x76, ROR, ZeroPageX, 6);
    test_opcode(0x6e, ROR, Absolute, 6);
    test_opcode(0x7e, ROR, AbsoluteX, 7);

    /* RTI */
    test_opcode(0x40, RTI, Implicit, 6);

    /* RTS */
    test_opcode(0x60, RTS, Implicit, 6);

    /* SBC */
    test_opcode(0xe9, SBC, Immediate, 2);
    test_opcode(0xe5, SBC, ZeroPage, 3);
    test_opcode(0xf5, SBC, ZeroPageX, 4);
    test_opcode(0xed, SBC, Absolute, 4);
    test_opcode(0xfd, SBC, AbsoluteX, 4);
    test_opcode(0xf9, SBC, AbsoluteY, 4);
    test_opcode(0xe1, SBC, IndirectX, 6);
    test_opcode(0xf1, SBC, IndirectY, 5);

    /* SEC */
    test_opcode(0x38, SEC, Implicit, 2);

    /* SED */
    test_opcode(0xf8, SED, Implicit, 2);

    /* SEI */
    test_opcode(0x78, SEI, Implicit, 2);

    /* STA */
    test_opcode(0x85, STA, ZeroPage, 3);
    test_opcode(0x95, STA, ZeroPageX, 4);
    test_opcode(0x8d, STA, Absolute, 4);
    test_opcode(0x9d, STA, AbsoluteX, 5);
    test_opcode(0x99, STA, AbsoluteY, 5);
    test_opcode(0x81, STA, IndirectX, 6);
    test_opcode(0x91, STA, IndirectY, 6);

    /* STX */
    test_opcode(0x86, STX, ZeroPage, 3);
    test_opcode(0x96, STX, ZeroPageY, 4);
    test_opcode(0x8e, STX, Absolute, 4);

    /* STY */
    test_opcode(0x84, STY, ZeroPage, 3);
    test_opcode(0x94, STY, ZeroPageX, 4);
    test_opcode(0x8c, STY, Absolute, 4);

    /* TAX */
    test_opcode(0xaa, TAX, Implicit, 2);

    /* TAY */
    test_opcode(0xa8, TAY, Implicit, 2);

    /* TSX */
    test_opcode(0xba, TSX, Implicit, 2);

    /* TXA */
    test_opcode(0x8a, TXA, Implicit, 2);

    /* TXS */
    test_opcode(0x9a, TXS, Implicit, 2);

    /* TYA */
    test_opcode(0x98, TYA, Implicit, 2);

    /* unofficial opcodes */
    test_opcode(0x1a, NOP, Implicit, 2);
    test_opcode(0x3a, NOP, Implicit, 2);
    test_opcode(0x5a, NOP, Implicit, 2);
    test_opcode(0x7a, NOP, Implicit, 2);
    test_opcode(0xda, NOP, Implicit, 2);
    test_opcode(0xfa, NOP, Implicit, 2);
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

#[test]
#[should_panic(expected = "Unknown opcode 0x2")]
fn test_unknown_opcode_causes_panic() {
    /* will have to update this when unofficial opcodes are finished */
    instruction::from_opcode(0x02);
}

fn testing_cpu() -> Box<CPU> {
    CPU::new(Box::new(CoreMemory::new_from_mapper(Rc::new(
        RefCell::new(Box::new(TestMapper::new())),
    ))))
}
