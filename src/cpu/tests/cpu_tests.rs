use crate::cpu::controller::CONTROLLER_ADDRESS;
use crate::cpu::tests::{cpu_for_testing, memory_for_testing, NoOpMemoryListener};
use crate::cpu::{tests, CPU};
use crate::ppu::PPURegister;
use crate::ppu::PPURegister::OAMDMA;
use crate::processor::Processor;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use winit::keyboard::{Key, NamedKey};

#[test]
fn test_cpu() {
    let cpu = &mut tests::cpu_for_testing();

    /* simple transition test: can we update via ADC, and a memory read? */
    cpu.accumulator = 0x05;
    cpu.write_mem(0x1234, 0x04); // value we will add
    cpu.index_x = 0x20;
    cpu.write_mem(0x1f, 0x34); // the address stored in zero page memory, lo
    cpu.write_mem(0x20, 0x12); // that address, hi
    cpu.program_counter = 0x8000;
    cpu.write_mem(0x8000, 0x61); // ADC, IndirectX
    cpu.write_mem(0x8001, 0xff); // wrapping add 0xff == subtract 1 from x
    let cycles = cpu.transition();
    assert_eq!(cpu.accumulator, 0x09); // added four to a
    assert_eq!(cycles, 6); // ADC IndirectX takes 6 cycles

    /* test nmi */
    cpu.set_nmi(true);
    assert_eq!(cpu.nmi_set(), true);
    cpu.status = 0;
    cpu.program_counter = 0x8000;
    cpu.s_register = 0x50; // so we can test how it affects stack
    cpu.write_mem(0x8000, 0xea); // NOP, but we won't run it
    cpu.write_mem(0xfffa, 0x00); // NMI handler lo byte
    cpu.write_mem(0xfffb, 0x90); // NMI handler hi byte
    cpu.write_mem(0x9000, 0x4e); // LSR absolute
    cpu.write_mem(0x9001, 0x05); // LSR absolute address to change lo
    cpu.write_mem(0x9002, 0x03); // LSR absolute address to change hi
    cpu.write_mem(0x0305, 0b0011_0011);
    let cycles = cpu.transition();
    assert_eq!(cycles, 6); // NOP takes two cycles, LSR Absolute takes 6
    assert_eq!(cpu.read_mem(0x0305), 0b0001_1001); // LSRed this value
    assert_eq!(cpu.program_counter, 0x9003); // next instruction in hypothetical NMI handler
    assert_eq!(cpu.s_register, 0x4d); // pushed 3 values onto stack
    assert_eq!(cpu.read_mem(0x0150), 0x80); // previous execution address hi byte
    assert_eq!(cpu.read_mem(0x014f), 0x00); // previous execution address lo byte
    assert_eq!(cpu.read_mem(0x014e), 0b0010_0000); // CPU status flags
    assert_eq!(cpu.nmi_set(), false); // when we're done, 'do nmi' flag is turned off

    /* test CPU as processor */
    assert_eq!(cpu.clock_speed(), 1_790_000); /* 1.79 MHz */
}

#[test]
fn test_set_key_source() {
    let mut cpu = cpu_for_testing();

    let key_source = Arc::new(Mutex::new(HashSet::new()));
    cpu.set_key_source(key_source.clone());

    {
        let mut unwrapped_key_source = key_source.lock().unwrap();
        unwrapped_key_source.insert(Key::from(NamedKey::Enter)); // start
        unwrapped_key_source.insert(Key::Character("z".parse().unwrap())); // B
    }
    cpu.write_mem(CONTROLLER_ADDRESS, 0x01);
    cpu.write_mem(CONTROLLER_ADDRESS, 0x00);
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // A
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 1); // B -- on
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // select
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 1); // start -- on
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // up
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // down
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // left
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 0); // right
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 1); // always report 1 now
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 1); // always report 1 now
    assert_eq!(cpu.read_mem(CONTROLLER_ADDRESS), 1); // always report 1 now
}

/* when performing an OAMDMA operation, it should take extra cycles */
#[test]
fn test_oamdma() {
    let addr = PPURegister::address(&OAMDMA);

    let mut memory = memory_for_testing();
    memory.register_listener(Rc::new(RefCell::new(NoOpMemoryListener::new(addr))));

    let mut cpu = CPU::new(Box::new(memory));

    cpu.write_mem(addr, 0x10);
    cpu.program_counter = 0xfff0;
    cpu.write_mem(0xfff0, 0xe8); // perform the two cycle INX instruction
    assert_eq!(cpu.transition(), 2 + 513); // 513 extra cycles for OAMDMA
}
