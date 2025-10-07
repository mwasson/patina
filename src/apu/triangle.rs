use crate::apu::length_counter::LengthCounter;
use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Triangle {
    sequencer: TriangleSequencer,
    length_counter: LengthCounter,
    linear_counter: LinearCounter,
    enabled: bool,
}

impl Triangle {
    pub fn initialize(memory: &Rc<RefCell<CoreMemory>>) -> Rc<RefCell<Triangle>> {
        let triangle_ref = Rc::new(RefCell::new(Triangle::new()));
        memory.borrow_mut().register_listener(triangle_ref.clone());

        triangle_ref
    }

    fn new() -> Triangle {
        Triangle {
            sequencer: TriangleSequencer::new(),
            length_counter: LengthCounter::new(),
            linear_counter: LinearCounter::new(),
            enabled: false,
        }
    }
    
    pub fn tick(&mut self, apu_counter: u16) {
        if !self.enabled {
            return;
        }
        let is_half_frame = apu_counter == 7456 || apu_counter == 14914;
        let is_quarter_frame = is_half_frame || apu_counter == 3728 || apu_counter == 11185;

        if is_quarter_frame {
            self.linear_counter.clock();

            if is_half_frame {
                self.length_counter.clock();
            }
        }

        /* TODO: a lot of duplicate code with the pulse tick */
        /* only clock sequence timer if both counters are non-zero */
        if self.linear_counter.is_active() && self.length_counter.is_active() {
            self.sequencer.clock();
        }
    }

    pub fn amplitude(&self) -> f32 {
        self.sequencer.amplitude() * self.length_counter.amplitude()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.length_counter.silence();
        }
        self.enabled = enabled;
    }
}

impl MemoryListener for Triangle {
    fn get_addresses(&self) -> Vec<u16> {
        [0x4008, 0x4009, 0x400a, 0x400b].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, _address: u16) -> u8 {
        memory.open_bus()
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        match address {
            0x4008 => {
                self.length_counter.set_halt(value & 0x80 != 0);
                self.linear_counter.load_lin_counter_data(value);
            }
            0x4009 => { /* unused */ }
            0x400a => {
                self.sequencer.timer.set_timer_lo(value);
            }
            0x400b => {
                self.sequencer.timer.set_timer_hi(value);
                self.length_counter.set_lc(value);
                self.linear_counter.set_reload();
            }
            _ => panic!("invalid triangle memory address"),
        }
    }
}

struct TriangleSequencer {
    sequence_index: u8,
    timer: Timer,
}

impl TriangleSequencer {
    fn new() -> TriangleSequencer {
        TriangleSequencer {
            sequence_index: 0,
            timer: Timer::new(),
        }
    }

    fn clock(&mut self) {
        if self.timer.clock() {
            self.sequence_index = (self.sequence_index + 1) % 32;
        }
    }

    fn amplitude(&self) -> f32 {
        let mut vol = self.sequence_index % 16;

        if self.sequence_index < 16 {
            vol = 15 - vol;
        }

        vol as f32
    }
}

struct LinearCounter {
    control_flag: bool,
    count: u8,
    reload_value: u8,
    reload_flag: bool,
}

impl LinearCounter {
    fn new() -> LinearCounter {
        LinearCounter {
            control_flag: false,
            count: 0,
            reload_value: 0,
            reload_flag: false,
        }
    }

    fn clock(&mut self) {
        if self.reload_flag {
            self.count = self.reload_value;
        } else if self.count != 0 {
            self.count -= 1;
        }

        if !self.control_flag {
            self.reload_flag = false;
        }
    }

    fn is_active(&self) -> bool {
        self.count > 0
    }

    fn set_reload(&mut self) {
        self.reload_flag = true;
    }

    fn load_lin_counter_data(&mut self, data: u8) {
        self.control_flag = data & 0x80 != 0;
        self.reload_value = data & 0x7f;
    }
}
