use crate::cpu::tests;
use crate::cpu::AddressingMode::{
    Absolute, AbsoluteX, AbsoluteY, Accumulator, Immediate, Implicit, Indirect, IndirectX,
    IndirectY, Relative, ZeroPage, ZeroPageX, ZeroPageY,
};

#[test]
fn test_addressing_modes() {
    let cpu = &mut tests::cpu_for_testing();

    cpu.write_mem(0x0010, 0x20);

    /* we can read memory through an addressing mode */
    assert_eq!(Absolute.deref(cpu, 0x10, 0x0), 0x20);
    /* we can write memory through an addressing mode */
    Absolute.write(cpu, 0x11, 0x0, 0x80);
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

    /* IndirectX */
    cpu.index_x = 0x10;
    cpu.write_mem(0x0030, 0xcd);
    cpu.write_mem(0x0031, 0xab);
    assert_eq!(
        IndirectX.resolve_address(cpu, 0x20, 0xaa /* unused */),
        0xabcd
    );
    /* when the address wraps over a page boundary, it goes back to the beginning, alas */
    cpu.write_mem(0x00ff, 0x11);
    cpu.write_mem(0x0100, 0x22);
    cpu.write_mem(0x0000, 0x33);
    cpu.index_x = 0x50;
    assert_eq!(
        IndirectX.resolve_address(cpu, 0xaf, 0xaa /* unused */),
        0x3311
    );

    /* IndirectY */
    cpu.index_y = 0x20;
    cpu.write_mem(0x0040, 0xbb);
    cpu.write_mem(0x0041, 0xaa);
    assert_eq!(
        IndirectY.resolve_address(cpu, 0x40, 0xbb /* unused */),
        0xaadb
    );
    /* also an issue with wrapping at the edge of the zero page */
    cpu.index_y = 0x30;
    cpu.write_mem(0x00ff, 0x11);
    cpu.write_mem(0x0100, 0x22);
    cpu.write_mem(0x0000, 0x33);
    assert_eq!(
        IndirectY.resolve_address(cpu, 0xff, 0xcc /* unused */),
        0x3341
    );

    /* Accumulator */
    cpu.accumulator = 0x50;
    assert_eq!(
        Accumulator.deref(cpu, 0xab /* unused */, 0xcd /* unused */),
        0x50
    );
    /* nothing special for accumulator using the boundary cross check */
    let mut extra_cycles = 0;
    assert_eq!(
        Accumulator.deref_check_boundary_cross(cpu, 0, 0, &mut extra_cycles),
        0x50
    );
    assert_eq!(extra_cycles, 0);

    /* Immediate */
    assert_eq!(Immediate.deref(cpu, 0x12, 0 /* unused */), 0x12);

    /* Not really anything to test with Implict, besides failure (see below) */
}

#[test]
fn test_addressing_modes_bytes_used() {
    /* how many bytes used by op codes using these addressing modes */
    assert_eq!(Implicit.get_bytes(), 1);
    assert_eq!(Accumulator.get_bytes(), 1);
    assert_eq!(Immediate.get_bytes(), 2);
    assert_eq!(ZeroPage.get_bytes(), 2);
    assert_eq!(ZeroPageX.get_bytes(), 2);
    assert_eq!(ZeroPageY.get_bytes(), 2);
    assert_eq!(Relative.get_bytes(), 2);
    assert_eq!(Absolute.get_bytes(), 3);
    assert_eq!(AbsoluteX.get_bytes(), 3);
    assert_eq!(AbsoluteY.get_bytes(), 3);
    assert_eq!(Indirect.get_bytes(), 3);
    assert_eq!(IndirectX.get_bytes(), 2);
    assert_eq!(IndirectY.get_bytes(), 2);
}

#[test]
#[should_panic(expected = "Immediate mode shouldn't look up in memory")]
fn test_cannot_use_immediate_directly() {
    Immediate.resolve_address(&mut tests::cpu_for_testing(), 0, 0);
}

#[test]
#[should_panic(expected = "Accumulator mode should never be directly referenced")]
fn test_cannot_use_accumulator_directly() {
    Accumulator.resolve_address(&mut tests::cpu_for_testing(), 0, 0);
}

#[test]
#[should_panic(expected = "Implicit mode should never be directly referenced")]
fn test_cannot_use_implicit_directly() {
    Implicit.resolve_address(&mut tests::cpu_for_testing(), 0, 0);
}
