use crate::apu::envelope::Envelope;
use crate::apu::length_counter::LengthCounter;
use crate::apu::sweep::Sweep;
use crate::apu::timer::Timer;
use crate::cpu::{CoreMemory, MemoryListener};
use std::cell::RefCell;
use std::rc::Rc;

/* waveform descriptions from https://www.nesdev.org/wiki/APU_Pulse */
const PULSE_DUTIES: [[bool; 8]; 4] = [
    [false, true, false, false, false, false, false, false],
    [false, true, true, false, false, false, false, false],
    [false, true, true, true, true, false, false, false],
    [true, false, false, true, true, true, true, true],
];

pub struct Pulse {
    first_address: u16,
    envelope: Envelope,
    length_counter: LengthCounter,
    sweep: Sweep,
    sequencer: PulseSequencer,
    enabled: bool,
}

impl Pulse {
    /* initializes a pulse, links it up as a listener on CoreMemory, and
     * wraps it appropriately
     */
    pub fn initialize(
        first_addr: u16,
        is_first_channel: bool,
        memory: &Rc<RefCell<CoreMemory>>,
    ) -> Rc<RefCell<Pulse>> {
        let pulse_ref = Rc::new(RefCell::new(Pulse::new(first_addr, is_first_channel)));
        memory.borrow_mut().register_listener(pulse_ref.clone());

        pulse_ref
    }

    /* private constructor */
    pub fn new(first_address: u16, is_second_channel: bool) -> Pulse {
        Pulse {
            first_address,
            envelope: Envelope::new(),
            length_counter: LengthCounter::new(),
            sweep: Sweep::new(is_second_channel),
            sequencer: PulseSequencer::new(),
            enabled: false,
        }
    }

    pub(crate) fn tick(&mut self, apu_counter: u16) {
        if !self.enabled {
            return;
        }
        /* on every tick, clock sequencer timer  */
        self.sequencer.clock();

        let is_half_frame = apu_counter == 7456 || apu_counter == 14914;
        let is_quarter_frame = is_half_frame || apu_counter == 3728 || apu_counter == 11185;

        if is_quarter_frame {
            self.envelope.clock();

            if is_half_frame {
                self.length_counter.clock();
                self.sweep.clock(&mut self.sequencer.timer);
            }
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.length_counter.silence();
        }
        self.enabled = enabled;
    }

    fn set_duty_envelope(&mut self, byte0: u8) {
        self.sequencer.duty = ((byte0 & 0xc0) >> 6) as usize; /* NB: does not change duty_index */
        self.envelope.set_envelope(byte0);
        self.length_counter.set_halt(byte0 & 0x20 != 0);
    }

    fn set_lc_timer_hi(&mut self, byte3: u8) {
        self.sequencer.timer.set_timer_hi(byte3);
        self.length_counter.set_lc(byte3);

        /* side-effects on write */
        self.sequencer.duty_index = 0;
        self.envelope.start();
    }

    pub fn amplitude(&self) -> f32 {
        self.length_counter.amplitude()
            * self.envelope.amplitude()
            * self.sequencer.amplitude()
            * self.sweep.amplitude()
    }
}

struct PulseSequencer {
    timer: Timer,
    duty: usize,
    duty_index: usize,
}

impl PulseSequencer {
    fn new() -> PulseSequencer {
        PulseSequencer {
            timer: Timer::new(),
            duty: 0,
            duty_index: 0,
        }
    }

    fn clock(&mut self) {
        if self.timer.clock() {
            self.duty_index = (self.duty_index + 1) % 8
        }
    }

    pub fn amplitude(&self) -> f32 {
        (self.timer.period >= 8 && PULSE_DUTIES[self.duty][self.duty_index]) as u8 as f32
    }
}

impl MemoryListener for Pulse {
    fn get_addresses(&self) -> Vec<u16> {
        let a = self.first_address;
        [a, a + 1, a + 2, a + 3].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, _address: u16) -> u8 {
        /* open bus, this shouldn't be done */
        memory.open_bus()
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        match address - self.first_address {
            0 => {
                self.set_duty_envelope(value);
            }
            1 => {
                self.sweep.set_sweep(value);
            }
            2 => {
                self.sequencer.timer.set_timer_lo(value);
            }
            3 => {
                self.set_lc_timer_hi(value);
            }
            _ => {
                panic!(
                    "APU instrument passed invalid memory address 0x{:x}",
                    address
                );
            }
        }
    }
}
