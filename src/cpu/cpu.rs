use crate::cpu;
use crate::cpu::{
    operation_from_memory, AddressingMode, Controller, CoreMemory, StatusFlag, INITIAL_PC_LOCATION,
    IRQ_HANDLER_LOCATION, NMI_HANDLER_LOCATION,
};
use crate::processor::Processor;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Add;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use winit::keyboard::Key;

pub struct CPU {
    pub accumulator: u8,
    pub index_x: u8,
    pub index_y: u8,
    pub s_register: u8,
    pub program_counter: u16,
    pub status: u8,
    memory: Rc<RefCell<CoreMemory>>,
    controller: Rc<RefCell<Controller>>,
}

impl Processor for CPU {
    fn clock_speed(&self) -> u64 {
        1790000 /* 1.79 MHz */
    } /* TODO constantize */
}

impl CPU {
    /* TODO comment */
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> Box<Self> {
        let controller = Rc::new(RefCell::new(Controller::new()));

        memory.borrow_mut().register_listener(controller.clone());

        /* set program counter to value in memory at this location */
        let mut result = Self {
            accumulator: 0x00,
            index_x: 0x00,
            index_y: 0x00,
            s_register: 0xff,
            program_counter: 0x00,
            status: (0x11) << 4,
            memory,
            controller,
        };

        result.program_counter =
            AddressingMode::Indirect.resolve_address_u16(&mut result, INITIAL_PC_LOCATION);

        Box::new(result)
    }

    /* performs one operation, and then returns when the next operation should run */
    #[inline(never)]
    pub fn transition(&mut self, start_time: Instant) -> Instant {
        if self.memory.borrow().nmi_set() {
            self.trigger_nmi();
        }

        let operation_loc = self.program_counter;
        /* TODO: what if this hits the top of program memory */
        let operation = operation_from_memory(
            self.read_mem(operation_loc),
            self.read_mem(operation_loc.wrapping_add(1)),
            self.read_mem(operation_loc.wrapping_add(2)),
        );

        operation.apply(self);

        start_time.add(self.cycles_to_duration(operation.realized_instruction.cycles))
    }

    pub fn update_flag(&mut self, flag: StatusFlag, new_val: bool) {
        flag.update_bool(self, new_val);
    }

    pub fn update_zero_neg_flags(&mut self, new_val: u8) {
        self.update_flag(StatusFlag::Zero, new_val == 0);
        self.update_flag(StatusFlag::Negative, new_val & 0x80 != 0);
    }

    pub fn push(&mut self, data: u8) {
        self.write_mem(cpu::addr(self.s_register, 0x01), data);
        self.s_register = self.s_register.wrapping_sub(1);
    }

    pub fn push_memory_loc(&mut self, mem_loc: u16) {
        self.push((mem_loc >> 8) as u8);
        self.push((mem_loc & 0xff) as u8);
    }

    pub fn pop_memory_loc(&mut self) -> u16 {
        let lower = self.pop();
        let upper = self.pop();

        cpu::addr(lower, upper)
    }

    pub fn pop(&mut self) -> u8 {
        self.s_register += 1;
        let value = self.read_mem(0x0100 + self.s_register as u16);
        value
    }

    pub fn irq_with_offset(&mut self, offset: u8) {
        self.push_memory_loc(self.program_counter.wrapping_add(offset as u16));
        self.push((self.status & !(1 << 4)) | (1 << 5));
        self.update_flag(StatusFlag::InterruptDisable, false);
        self.program_counter = self.read_mem16(IRQ_HANDLER_LOCATION);
    }
    pub fn addr_from_mem16(&mut self, lo_byte_addr: u16) -> u16 {
        self.read_mem16(lo_byte_addr)
    }

    fn trigger_nmi(&mut self) {
        self.memory.borrow_mut().set_nmi(false);
        /* push PC onto stack */
        self.push_memory_loc(self.program_counter);
        /* push processor status register on stack */
        self.push((self.status & !(1 << 4)) | (1 << 5));
        /* read NMI handler address from 0xFFFA/0xFFFB and jump to that address*/
        self.program_counter =
            AddressingMode::Indirect.resolve_address_u16(self, NMI_HANDLER_LOCATION);
    }

    pub fn write_mem(&mut self, addr: u16, data: u8) {
        self.memory.borrow_mut().write(addr, data);
    }

    pub fn read_mem(&self, addr: u16) -> u8 {
        self.memory.borrow().read(addr)
    }

    pub fn read_mem16(&mut self, addr: u16) -> u16 {
        self.memory.borrow().read16(addr)
    }

    pub fn set_key_source(&mut self, keys: Arc<Mutex<HashSet<Key>>>) {
        self.controller.borrow_mut().set_key_source(keys);
    }
}
