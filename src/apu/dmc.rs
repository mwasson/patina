use std::cell::RefCell;
use std::rc::Rc;
use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};

/* different for PAL */
const NTSC_RATE_MAP : [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214,
    190, 160, 142, 128, 106,  84,  72,  54
];

pub struct DMC {
    timer: Timer,
    memory: Rc<RefCell<CoreMemory>>,

    sample_buffer: Option<u8>,
    sample_address: u8, /* offset from 0xC000 */
    sample_length: u8, /* actual length is (sample_length * 16) + 1 bytes */
    silence_flag: bool,
    shift_register: u8,
    volume: u8,
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u16,
    enabled: bool,
}

impl DMC {
    pub fn initialize(memory: &Rc<RefCell<CoreMemory>>) -> Rc<RefCell<DMC>> {
        let dmc_ref = Rc::new(RefCell::new(DMC::new(memory)));
        memory.borrow_mut().register_listener(dmc_ref.clone());

        dmc_ref
    }

    pub fn new(memory: &Rc<RefCell<CoreMemory>>) -> DMC {
        let mut timer = Timer::new();
        timer.set_period(8);

        DMC {
            timer,
            memory: memory.clone(),
            sample_buffer: None,
            sample_address: 0,
            sample_length: 0,
            silence_flag: true,
            shift_register: 0,
            volume: 0,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            enabled: false,
        }
    }
    
    pub fn tick(&mut self, apu_counter: u16) {
        if !self.enabled {
            return;
        }
        if !self.silence_flag {
            if self.shift_register & 1 == 1 {
                if self.volume <= 125 {
                    self.volume += 2;
                }
            }  else {
                if self.volume >= 2 {
                    self.volume -= 2;
                }
            }
        }
        /* TODO clock shift register??? */

        if self.timer.clock() {
            match self.sample_buffer {
                Some(sample) => {
                    self.silence_flag = false;
                    self.shift_register = sample;
                    self.sample_buffer = None;
                    /* TODO read memory here? */
                }
                None => {
                    self.silence_flag = true;
                }
            }
        }
    }

    pub fn amplitude(&self) -> f32 {
        self.volume as f32 / 15.0
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        /* TODO: DMC behavior for being disabled is more complex than others */
        self.enabled = enabled;
    }
}

impl MemoryListener for DMC {
    fn get_addresses(&self) -> Vec<u16> {
        [0x4010,0x4011,0x4012,0x4013].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8 {
        memory.read(address)
    }

    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8) {
        match address {
            0x4010 => {
                self.irq_enabled = value & 0x80 != 0; /* TODO: clear interrupt flag when cleared */
                self.loop_flag = value & 0x40 != 0;
                self.rate_index = NTSC_RATE_MAP[(value & 0x0f) as usize];
            }
            0x4011 => {
                /* writes value directly, but usually load a sample instead */
                self.volume = value;
            }
            0x4012 => {
                self.sample_address = value;
            }
            0x4013 => {
                self.sample_length = value;
            }
            _ => unreachable!()
        }
    }
}