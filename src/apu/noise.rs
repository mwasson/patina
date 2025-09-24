use std::cell::RefCell;
use std::rc::Rc;
use crate::apu::envelope::Envelope;
use crate::apu::length_counter::LengthCounter;
use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};

/* these values are different on PAL */
const NTSC_NOISE_PERIODS : [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160,
    202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub struct Noise {
    envelope: Envelope,
    length_counter: LengthCounter,
    timer: Timer,

    shift_register: u16, /* NB: only use the lower 15 bits */
    mode_flag: bool,
}

impl Noise {
    pub fn initialize(memory: &Rc<RefCell<CoreMemory>>) -> Rc<RefCell<Noise>> {
        let noise_ref = Rc::new(RefCell::new(Noise::new()));
        memory.borrow_mut().register_listener(noise_ref.clone());

        noise_ref
    }

    fn new() -> Noise {
        Noise {
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            timer: Timer::new(),
            shift_register: 0,
            mode_flag: false,
        }
    }

    pub fn tick(&mut self, apu_counter: u16) {
        if self.timer.clock() {
            let compare_shift = if self.mode_flag { 6 } else { 1 };
            let feedback = (self.shift_register ^ (self.shift_register >> compare_shift)) & 1;
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        }

        if apu_counter == 7456 || apu_counter == 14914 {
            self.length_counter.clock();
        }
    }

    pub fn amplitude(&self) -> f32 {
        self.envelope.amplitude() * self.length_counter.amplitude() * (self.shift_register & 1) as f32
    }
}

impl MemoryListener for Noise {
    fn get_addresses(&self) -> Vec<u16> {
        [0x400c, 0x400e, 0x400f].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8 {
        memory.read_no_listen(address)
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        match address {
            0x400c => {
                self.length_counter.set_halt(value & 0x20 != 0);
                self.envelope.set_envelope(value)
                /* TODO constant volume/envelope flag */
                /* TODO volume/envelope divider period */
            }
            0x400e => {
                self.mode_flag = value & 0x80 != 0;
                self.timer.set_period(NTSC_NOISE_PERIODS[(value & 0x0f) as usize]);
            }
            0x400f => {
                self.length_counter.set_lc(value);
                self.envelope.start();
            }
            _ => unreachable!(),
        }
    }
}