use crate::cpu::instruction::Instruction;
use crate::cpu::instruction::Instruction::*;
use crate::cpu::tests::cpu_for_testing;
use crate::cpu::AddressingMode::*;
use crate::cpu::StatusFlag::*;
use crate::cpu::{from_opcode, instruction, tests, AddressingMode, CPU};

#[test]
fn test_instructions() {
    let cpu = &mut tests::cpu_for_testing();

    /* ADC */
    /* simple addition test */
    ADC.apply(cpu, &Immediate, 0x05, 0x0);
    /* 0x0 + 0x5 = 0x5 */
    assert_eq!(cpu.accumulator, 5);
    assert_eq!(Carry.is_set(cpu), false); // no carry
    assert_eq!(Zero.is_set(cpu), false); // not zero
    assert_eq!(Overflow.is_set(cpu), false); // no overflow
    assert_eq!(Negative.is_set(cpu), false); // positive result
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
    BRK.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
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
    CLC.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0b11111110); /* no effect if carry already clear */
    cpu.status = 1;
    CLC.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0); /* clears carry */

    /* CLD */
    /* TODO clean up use of flags */
    cpu.status = 0b1111_0111;
    CLD.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0b1111_0111); /* no effect if decimal already clear */
    cpu.status = 0b0000_1000;
    CLD.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0); /* clears decimal */

    /* CLI */
    /* TODO clean up use of flags */
    cpu.status = 0b1111_1011;
    CLI.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0b1111_1011); /* no effect if interrupt disable already clear */
    cpu.status = 0b0000_0100;
    CLI.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0); /* clears interrupt disable */

    /* CLV */
    /* TODO clean up use of flags */
    cpu.status = 0b1011_1111;
    CLV.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0b1011_1111); /* no effect if overflow already clear */
    cpu.status = 0b0100_0000;
    CLV.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.status, 0); /* clears overflow */

    /* CMP */
    cpu.accumulator = 0x10; /* should be in function but leads to borrowing issues */
    test_compare_instruction(cpu, &CMP);
    assert_eq!(cpu.accumulator, 0x10); // compare operations do not affect the register

    /* CPX */
    cpu.index_x = 0x10; /* should be in function but leads to borrowing issues */
    test_compare_instruction(cpu, &CPX);
    assert_eq!(cpu.index_x, 0x10); // compare operations do not affect the register

    /* CPY */
    cpu.index_y = 0x10; /* should be in function but leads to borrowing issues */
    test_compare_instruction(cpu, &CPY);
    assert_eq!(cpu.index_y, 0x10); // compare operations do not affect the register

    /* DEC */
    /* setting these values for testing */
    Carry.update_bool(cpu, true);
    Overflow.update_bool(cpu, false);
    cpu.write_mem(0x1000, 0xc0);
    DEC.apply(cpu, &Absolute, 0x00, 0x10);
    assert_eq!(cpu.read_mem(0x1000), 0xbf);
    assert_eq!(Zero.is_set(cpu), false); // not zero
    assert_eq!(Negative.is_set(cpu), true); // is negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected
    cpu.write_mem(0x1000, 0x01);
    DEC.apply(cpu, &Absolute, 0x00, 0x10);
    assert_eq!(cpu.read_mem(0x1000), 0x00);
    assert_eq!(Zero.is_set(cpu), true); // not zero
    assert_eq!(Negative.is_set(cpu), false); // not negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected

    /* DEX */
    /* setting these values for testing */
    Carry.update_bool(cpu, true);
    Overflow.update_bool(cpu, false);
    cpu.index_x = 0xc0;
    DEX.apply(cpu, &Implicit, 0x00, 0x10);
    assert_eq!(cpu.index_x, 0xbf);
    assert_eq!(Zero.is_set(cpu), false); // not zero
    assert_eq!(Negative.is_set(cpu), true); // is negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected
    cpu.index_x = 0x01;
    DEX.apply(cpu, &Implicit, 0x00, 0x10);
    assert_eq!(cpu.index_x, 0x00);
    assert_eq!(Zero.is_set(cpu), true); // not zero
    assert_eq!(Negative.is_set(cpu), false); // not negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected

    /* DEY */
    /* setting these values for testing */
    Carry.update_bool(cpu, true);
    Overflow.update_bool(cpu, false);
    cpu.index_y = 0xc0;
    DEY.apply(cpu, &Implicit, 0x00, 0x10);
    assert_eq!(cpu.index_y, 0xbf);
    assert_eq!(Zero.is_set(cpu), false); // not zero
    assert_eq!(Negative.is_set(cpu), true); // is negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected
    cpu.index_y = 0x01;
    DEY.apply(cpu, &Implicit, 0x00, 0x10);
    assert_eq!(cpu.index_y, 0x00);
    assert_eq!(Zero.is_set(cpu), true); // not zero
    assert_eq!(Negative.is_set(cpu), false); // not negative
    assert_eq!(Carry.is_set(cpu), true); // unaffected
    assert_eq!(Overflow.is_set(cpu), false); // unaffected

    /* EOR */
    cpu.accumulator = 0b0011_0011;
    let extra_cycles = EOR.apply(cpu, &Immediate, 0b1010_1010, 0x10 /* unused */);
    assert_eq!(cpu.accumulator, 0b1001_1001);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), true);
    assert_eq!(extra_cycles, 0);
    /* test boundary crossing penalty using AbsoluteX */
    cpu.index_x = 0;
    /* reading from 0x01f0 + 0x0 = 0x01f0, no boundary crossing */
    let extra_cycles = EOR.apply(cpu, &AbsoluteX, 0xf0, 0x01);
    assert_eq!(extra_cycles, 0);
    cpu.index_x = 0xff;
    /* reading from 0x01f0 + 0x00ff = 0x02ef, boundary crossing */
    let extra_cycles = EOR.apply(cpu, &AbsoluteX, 0xf0, 0x01);
    assert_eq!(extra_cycles, 1);

    /* INC */
    cpu.write_mem(0x0120, 0xff);
    INC.apply(cpu, &Absolute, 0x20, 0x01);
    assert_eq!(cpu.read_mem(0x0120), 0x0);
    assert_eq!(Zero.is_set(cpu), true);
    assert_eq!(Negative.is_set(cpu), false);

    /* INX */
    cpu.index_x = 0;
    INX.apply(
        cpu, &Implicit, 0xf0, /* unused */
        0x01, /* unused */
    );
    assert_eq!(cpu.index_x, 0x1); /* x = x + 1 */
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), false);

    /* INX */
    cpu.index_y = 0x7f;
    INY.apply(
        cpu, &Implicit, 0xf0, /* unused */
        0x01, /* unused */
    );
    assert_eq!(cpu.index_y, 0x80); /* y = y + 1 */
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), true);

    /* JMP */
    cpu.program_counter = 0xffff;
    JMP.apply(cpu, &Absolute, 0x50, 0x90);
    assert_eq!(cpu.program_counter, 0x9050);
    cpu.write_mem(0x0200, 0xab);
    cpu.write_mem(0x02fe, 0xcd);
    cpu.write_mem(0x02ff, 0xef);
    cpu.write_mem(0x0300, 0x12);
    /* special indirect addressing mode: normal case */
    JMP.apply(cpu, &Indirect, 0xfe, 0x02);
    assert_eq!(cpu.program_counter, 0xefcd);
    /* if it crosses a page boundary, it does the wrong thing and wraps to beginning of page */
    JMP.apply(cpu, &Indirect, 0xff, 0x02);
    assert_eq!(cpu.program_counter, 0xabef); /* NB not 0x12ef */

    /* JSR */
    cpu.s_register = 0x80;
    cpu.program_counter = 0xf123;
    JSR.apply(cpu, &Absolute, 0xef, 0xcd);
    assert_eq!(cpu.program_counter, 0xcdef);
    assert_eq!(cpu.s_register, 0x80 - 2);
    assert_eq!(cpu.read_mem(0x0180), 0xf1); // high byte of old PC + 2
    assert_eq!(cpu.read_mem(0x017f), 0x25); // low byte of old PC + 2
                                            /* check it works with wrapping addresses */
    cpu.s_register = 0x80;
    cpu.program_counter = 0xf1ff;
    JSR.apply(cpu, &Absolute, 0xef, 0xcd);
    assert_eq!(cpu.read_mem(0x0180), 0xf2); // high byte of old PC + 2
    assert_eq!(cpu.read_mem(0x017f), 0x01); // low byte of old PC + 2

    /* LDA */
    cpu.accumulator = 0;
    LDA.apply(cpu, &Immediate, 0xff, 0x01 /* unused */);
    assert_eq!(cpu.accumulator, 0xff);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), true);

    /* LDX */
    cpu.index_x = 0x10;
    LDX.apply(cpu, &Immediate, 0x0, 0x01 /* unused */);
    assert_eq!(cpu.index_x, 0x0);
    assert_eq!(Zero.is_set(cpu), true);
    assert_eq!(Negative.is_set(cpu), false);

    /* LDY */
    cpu.index_y = 0;
    LDY.apply(cpu, &Immediate, 0x20, 0x01 /* unused */);
    assert_eq!(cpu.index_y, 0x20);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), false);

    /* LSR */
    cpu.accumulator = 0b1010_1111;
    LSR.apply(
        cpu,
        &Accumulator,
        0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.accumulator, 0b0101_0111);
    assert_eq!(Carry.is_set(cpu), true);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), false);

    /* NOP */
    // well, it doesn't do anything ... */
    cpu.accumulator = 0xab;
    cpu.index_x = 0xcd;
    cpu.index_y = 0xef;
    NOP.apply(
        cpu, &Implicit, 0x00, /* unused */
        0x12, /* unused */
    );
    /* check registers didn't change */
    assert_eq!(cpu.accumulator, 0xab);
    assert_eq!(cpu.index_x, 0xcd);
    assert_eq!(cpu.index_y, 0xef);

    /* ORA */
    cpu.accumulator = 0b1100_1100;
    ORA.apply(cpu, &Immediate, 0b1010_1010, 0x01 /* unused */);
    assert_eq!(cpu.accumulator, 0b1110_1110);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), true);

    /* PHA */
    cpu.s_register = 0x90;
    cpu.accumulator = 0x15;
    PHA.apply(
        cpu, &Implicit, 0xab, /* unused */
        0xcd, /* unused */
    );
    assert_eq!(cpu.s_register, 0x90 - 1);
    assert_eq!(cpu.read_mem(0x0190), 0x15);

    /* PHP */
    cpu.status = 0b0011_0011;
    cpu.s_register = 0x80;
    PHP.apply(
        cpu, &Implicit, 0xaa, /* unused */
        0xbb, /* unused */
    );
    assert_eq!(cpu.s_register, 0x80 - 1);
    assert_eq!(cpu.read_mem(0x0180), 0b0011_0011); /* same as current status */
    /* now guarantee bit 5 is 1 (always should be), and so is B flag */
    cpu.status = 0b0000_0000;
    PHP.apply(
        cpu, &Implicit, 0xaa, /* unused */
        0xbb, /* unused */
    );
    assert_eq!(cpu.s_register, 0x80 - 2);
    assert_eq!(cpu.read_mem(0x017f), 0b0011_0000);

    /* PLA */
    cpu.accumulator = 0;
    cpu.s_register = 0xfe;
    cpu.write_mem(0x01ff, 0x20);
    PLA.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.accumulator, 0x20);
    assert_eq!(cpu.s_register, 0xff);
    assert_eq!(Zero.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), false);

    /* PLP */
    cpu.s_register = 0x80;
    cpu.status = 0;
    cpu.write_mem(0x181, 0b1011_0101);
    PLP.apply(cpu, &Implicit, 0, 0);
    /* ignores the B flag and always-on bit (cheating here by not setting that) */
    assert_eq!(cpu.status, 0b1000_0101);

    /* ROL */
    /* rotate left, new carry is zero, old carry is one */
    cpu.accumulator = 0b0011_0101;
    Carry.update_bool(cpu, true);
    ROL.apply(cpu, &Accumulator, 0, 0);
    assert_eq!(cpu.accumulator, 0b01101011);
    assert_eq!(Carry.is_set(cpu), false);
    /* rotate left, new carry is one, old carry is zero */
    cpu.write_mem(0x0123, 0b1111_0000);
    Carry.update_bool(cpu, false);
    ROL.apply(cpu, &Absolute, 0x23, 0x01);
    assert_eq!(cpu.read_mem(0x0123), 0b1110_0000);
    assert_eq!(Carry.is_set(cpu), true);

    /* ROR */
    /* rotate right, new carry is zero, old carry is one */
    cpu.accumulator = 0b0011_0100;
    Carry.update_bool(cpu, true);
    ROR.apply(cpu, &Accumulator, 0, 0);
    assert_eq!(cpu.accumulator, 0b1001_1010);
    assert_eq!(Carry.is_set(cpu), false);
    /* rotate right, new carry is one, old carry is zero */
    cpu.write_mem(0x0123, 0b1111_0001);
    Carry.update_bool(cpu, false);
    ROR.apply(cpu, &Absolute, 0x23, 0x01);
    assert_eq!(cpu.read_mem(0x0123), 0b0111_1000);
    assert_eq!(Carry.is_set(cpu), true);

    /* RTI */
    cpu.s_register = 0x40;
    cpu.status = 0b0011_0000;
    cpu.write_mem(0x0141, 0b1100_1010); // status flags; should ignore 4 & 5
    cpu.write_mem(0x0142, 0x30); // return address low byte
    cpu.write_mem(0x0143, 0xff); // return address high byte
    RTI.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.program_counter, 0xff30); // NB: other tests cover that CPU stays at this addr
    assert_eq!(cpu.s_register, 0x40 + 3);
    assert_eq!(cpu.status, 0b1111_1010);

    /* RTS */
    cpu.s_register = 0x60;
    cpu.write_mem(0x0161, 0x50); // return address low byte
    cpu.write_mem(0x0162, 0xff); // return address high byte
    RTS.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.program_counter, 0xff51); // increments one from memory loc
    assert_eq!(cpu.s_register, 0x60 + 2);

    /* SBC */
    /* carry zero */
    cpu.accumulator = 0x20;
    Carry.update_bool(cpu, false);
    SBC.apply(cpu, &Immediate, 0x30, 0);
    assert_eq!(cpu.accumulator, !0x11 + 1); // 0x20 - 0x30 - ~0 = 0x20 - 0x30 - 1 = -(0x11)
    assert_eq!(Carry.is_set(cpu), false); // overflowed below zero, so would be carry op but flag is false
    assert_eq!(Negative.is_set(cpu), true);
    assert_eq!(Overflow.is_set(cpu), false);
    /* carry one */
    cpu.accumulator = 0x20;
    Carry.update_bool(cpu, true);
    SBC.apply(cpu, &Immediate, 0x05, 0);
    assert_eq!(cpu.accumulator, 0x1b); // 0x20 - 0x05 - ~1 = 0x20 - 0x05 = 0x1c
    assert_eq!(Carry.is_set(cpu), true); // subtraction w/o carry, but it's "true carry"
    assert_eq!(Overflow.is_set(cpu), false);
    assert_eq!(Negative.is_set(cpu), false);

    /* SEC */
    Carry.update_bool(cpu, false);
    SEC.apply(cpu, &Implicit, 0, 0);
    assert_eq!(Carry.is_set(cpu), true);

    /* SED */
    Decimal.update_bool(cpu, false);
    SED.apply(cpu, &Implicit, 0, 0);
    assert_eq!(Decimal.is_set(cpu), true);

    /* SEI */
    InterruptDisable.update_bool(cpu, false);
    SEI.apply(cpu, &Implicit, 0, 0);
    assert_eq!(InterruptDisable.is_set(cpu), true);

    /* STA */
    cpu.accumulator = 0xaa;
    STA.apply(cpu, &Absolute, 0xff, 0x00);
    assert_eq!(cpu.read_mem(0x00ff), 0xaa);

    /* STX */
    cpu.index_x = 0xbb;
    STX.apply(cpu, &Absolute, 0xff, 0x00);
    assert_eq!(cpu.read_mem(0x00ff), 0xbb);

    /* STY */
    cpu.index_y = 0xcc;
    STY.apply(cpu, &Absolute, 0xff, 0x00);
    assert_eq!(cpu.read_mem(0x00ff), 0xcc);

    /* TAX */
    cpu.accumulator = 0x12;
    cpu.index_x = 0;
    TAX.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.index_x, 0x12);

    /* TAY */
    cpu.accumulator = 0x12;
    cpu.index_y = 0;
    TAY.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.index_y, 0x12);

    /* TSX */
    cpu.index_x = 0;
    cpu.s_register = 0x30;
    TSX.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.index_x, 0x30);

    /* TXA */
    cpu.accumulator = 0;
    cpu.index_x = 0x40;
    TXA.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.accumulator, 0x40);

    /* TXS */
    cpu.s_register = 0;
    cpu.index_x = 0x50;
    TXS.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.s_register, 0x50);

    /* TYA */
    cpu.accumulator = 0;
    cpu.index_y = 0x60;
    TYA.apply(cpu, &Implicit, 0, 0);
    assert_eq!(cpu.index_y, 0x60);
}

fn test_compare_instruction(cpu: &mut CPU, instruction: &Instruction) {
    instruction.apply(cpu, &Immediate, 0x10, 0xff /* unused */);
    assert_eq!(Carry.is_set(cpu), true); // carry on equality
    assert_eq!(Zero.is_set(cpu), true); // is zero
    assert_eq!(Negative.is_set(cpu), false); // is zero
    instruction.apply(cpu, &Immediate, 0x15, 0xff /* unused */);
    assert_eq!(Carry.is_set(cpu), false); // no carry when A < memory
    assert_eq!(Zero.is_set(cpu), false); // is negative
    assert_eq!(Negative.is_set(cpu), true); // is negative
    instruction.apply(cpu, &Immediate, 0x05, 0xff /* unused */);
    assert_eq!(Carry.is_set(cpu), true); // carry when A > memory
    assert_eq!(Zero.is_set(cpu), false); // is positive
    assert_eq!(Negative.is_set(cpu), false); // is positive
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

#[test]
fn realized_jump_instructions_dont_increment_pc() {
    let cpu = &mut cpu_for_testing();

    cpu.program_counter = 0;
    let realized_jmp = from_opcode(0x4c);
    realized_jmp.apply(cpu, 0x00, 0x20);
    assert_eq!(cpu.program_counter, 0x2000);

    cpu.program_counter = 0;
    let realized_jsr = from_opcode(0x20);
    realized_jsr.apply(cpu, 0x00, 0x30);
    assert_eq!(cpu.program_counter, 0x3000);

    let realized_rts = from_opcode(0x60);
    cpu.program_counter = 0;
    cpu.s_register = 0x50;
    cpu.write_mem(0x0151, 0x22);
    cpu.write_mem(0x0152, 0x33);
    realized_rts.apply(cpu, 0xff /* unused */, 0xff /* unused */);
    assert_eq!(cpu.program_counter, 0x3323);

    let realized_rti = from_opcode(0x40);
    cpu.program_counter = 0;
    cpu.s_register = 0x50;
    cpu.write_mem(0x0151, 0b0011_0000);
    cpu.write_mem(0x0152, 0x22);
    cpu.write_mem(0x0153, 0x33);
    realized_rti.apply(cpu, 0xff /* unused */, 0xff /* unused */);
    assert_eq!(cpu.program_counter, 0x3322);

    let realized_brk = from_opcode(0x00);
    cpu.program_counter = 0;
    cpu.write_mem(0xfffe, 0x77);
    cpu.write_mem(0xffff, 0x66);
    realized_brk.apply(cpu, 0xff, 0xff);
    assert_eq!(cpu.program_counter, 0x6677);

    /* ...but make sure it does the right thing for regular instructions */
    cpu.program_counter = 0xabcd;
    cpu.index_x = 0x10;
    let realized_inx = from_opcode(0xe8);
    realized_inx.apply(cpu, 0x12 /* unused */, 0x34 /* unused */);
    assert_eq!(cpu.index_x, 0x11);
    assert_eq!(cpu.program_counter, 0xabce); /* INX is one byte long */
}
