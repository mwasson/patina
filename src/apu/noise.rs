use crate::apu::envelope::Envelope;
use crate::apu::length_counter::LengthCounter;
use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};
use std::cell::RefCell;
use std::rc::Rc;

/* these values are different on PAL */
const NTSC_NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct Noise {
    envelope: Envelope,
    length_counter: LengthCounter,
    timer: Timer,

    shift_register: u16, /* NB: only use the lower 15 bits */
    mode_flag: bool,
    enabled: bool,
}

impl Noise {
    pub fn new() -> Noise {
        Noise {
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            timer: Timer::new(),
            shift_register: 1,
            mode_flag: false,
            enabled: false,
        }
    }

    pub fn tick(&mut self, apu_counter: u16) {
        if !self.enabled {
            return;
        }
        if self.timer.clock() {
            let compare_shift = if self.mode_flag { 6 } else { 1 };
            let feedback = (self.shift_register ^ (self.shift_register >> compare_shift)) & 1;
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        }

        let is_half_frame = apu_counter == 7456 || apu_counter == 14914;
        let is_quarter_frame = is_half_frame || apu_counter == 3728 || apu_counter == 11185;

        if is_quarter_frame {
            self.envelope.clock();
        }

        if is_half_frame || is_quarter_frame {
            self.length_counter.clock();
        }
    }

    pub fn amplitude(&self) -> f32 {
        self.envelope.amplitude()
            * self.length_counter.amplitude()
            * (self.shift_register & 1 != 0) as u8 as f32
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.length_counter.silence();
        }
        self.enabled = enabled;
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x400c => {
                // println!("400c {:x}", value);
                self.length_counter.set_halt(value & 0x20 != 0);
                self.envelope.set_envelope(value);
                /* TODO constant volume/envelope flag */
                /* TODO volume/envelope divider period */
            }
            0x400d => { /* unused */ }
            0x400e => {
                self.mode_flag = value & 0x80 != 0;
                self.timer
                    .set_period(NTSC_NOISE_PERIODS[(value & 0x0f) as usize] / 2);
            }
            0x400f => {
                self.length_counter.set_lc(value);
                self.envelope.start();
            }
            _ => unreachable!(),
        }
    }
}
