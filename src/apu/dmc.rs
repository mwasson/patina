use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};
use std::cell::RefCell;
use std::rc::Rc;

/* different for PAL */
const NTSC_RATE_MAP: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

pub struct DMC {
    timer: Timer,
    memory: Rc<RefCell<CoreMemory>>,
    bits_remaining: u8,
    sample_buffer: Option<u8>,
    sample_address: u16,
    current_address: u16,
    sample_length: u16,
    sample_bytes_remaining: u16,
    silence_flag: bool,
    shift_register: u8,
    volume: u8,
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u16,
    enabled: bool,
}

impl DMC {
    #[allow(dead_code)]
    pub fn initialize(memory: &Rc<RefCell<CoreMemory>>) -> Rc<RefCell<DMC>> {
        let dmc_ref = Rc::new(RefCell::new(DMC::new(memory)));
        memory.borrow_mut().register_listener(dmc_ref.clone());

        dmc_ref
    }

    pub fn new(memory: &Rc<RefCell<CoreMemory>>) -> DMC {
        let timer = Timer::new();

        DMC {
            timer,
            memory: memory.clone(),
            bits_remaining: 0,
            sample_buffer: None,
            sample_address: 0,
            current_address: 0,
            sample_length: 0,
            sample_bytes_remaining: 0,
            silence_flag: true,
            shift_register: 0,
            volume: 0,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            enabled: false,
        }
    }

    pub fn tick(&mut self, _apu_counter: u16) {
        if !self.enabled {
            return;
        }

        if self.sample_buffer.is_none() && self.sample_bytes_remaining > 0 {
            self.sample_buffer = Some(self.memory.borrow_mut().read(self.current_address));
            if self.current_address == 0xffff {
                self.current_address = 0x8000;
            } else {
                self.current_address += 1;
            }
            self.sample_bytes_remaining -= 1;
            if self.sample_bytes_remaining == 0 && self.loop_flag {
                self.current_address = self.sample_address;
                self.sample_bytes_remaining = self.sample_length;
            }
            /* TODO IRQ interrupt */
        }

        if self.timer.clock() {
            if !self.silence_flag {
                if self.shift_register & 1 != 0 {
                    if self.volume < 125 {
                        self.volume += 2;
                    }
                } else if self.volume > 1 {
                    self.volume -= 2;
                }
            }
            /* TODO clock shift register */
            self.shift_register >>= 1;
            self.bits_remaining -= 1;
            if self.bits_remaining == 0 {
                if self.sample_buffer.is_none() {
                    self.silence_flag = true;
                } else {
                    self.silence_flag = false;
                    self.shift_register = self.sample_buffer.unwrap();
                    self.sample_buffer = None;
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
        [0x4010, 0x4011, 0x4012, 0x4013].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8 {
        memory.read(address)
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        match address {
            0x4010 => {
                self.irq_enabled = value & 0x80 != 0; /* TODO: clear interrupt flag when cleared */
                self.loop_flag = value & 0x40 != 0;
                /* map from number of CPU cycles to number of APU cycles */
                self.rate_index = NTSC_RATE_MAP[(value & 0x0f) as usize] >> 2;
                self.timer.set_period(self.rate_index);
            }
            0x4011 => {
                /* writes value directly, but usually load a sample instead */
                self.volume = value;
            }
            0x4012 => {
                /* memory value is offset from 0xc000, aligned to 64 byte chunks */
                self.sample_address = 0xc000 | ((value as u16) << 6);
                self.current_address = self.sample_address;
            }
            0x4013 => {
                /* actual length is (sample_length * 16) + 1 bytes */
                self.sample_length = ((value as u16) << 4) | 1;
                self.sample_bytes_remaining = self.sample_length;
            }
            _ => unreachable!(),
        }
    }
}
