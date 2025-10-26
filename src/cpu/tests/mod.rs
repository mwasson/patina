use crate::cpu::tests::test_mapper::TestMapper;
use crate::cpu::{CoreMemory, MemoryListener, CPU};
use std::cell::RefCell;
use std::rc::Rc;

mod addressing_mode_tests;
mod controller_tests;
mod cpu_tests;
mod instruction_tests;
mod memory_tests;
mod test_mapper;

fn memory_for_testing() -> CoreMemory {
    CoreMemory::new_from_mapper(Rc::new(RefCell::new(Box::new(TestMapper::new()))))
}

fn cpu_for_testing() -> Box<CPU> {
    CPU::new(Box::new(CoreMemory::new_from_mapper(Rc::new(
        RefCell::new(Box::new(TestMapper::new())),
    ))))
}

struct NoOpMemoryListener {
    addr: u16,
}

impl NoOpMemoryListener {
    fn new(addr: u16) -> NoOpMemoryListener {
        NoOpMemoryListener { addr }
    }
}

impl MemoryListener for NoOpMemoryListener {
    fn get_addresses(&self) -> Vec<u16> {
        vec![self.addr]
    }

    fn read(&mut self, _memory: &CoreMemory, _address: u16) -> u8 {
        0 /* ignored */
    }

    fn write(&mut self, _memory: &CoreMemory, _address: u16, _value: u8) {
        /* noop */
    }
}
