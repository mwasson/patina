use crate::cpu::tests::test_mapper::TestMapper;
use crate::cpu::{CoreMemory, CPU};
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
