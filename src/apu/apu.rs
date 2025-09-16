use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::CoreMemory;

pub struct APU {
    memory: Rc<RefCell<CoreMemory>>
}

impl APU {
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> APU {
        APU {
            memory
        }
    }
}