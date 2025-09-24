use std::cell::RefCell;
use std::rc::Rc;
use crate::cpu::{CoreMemory, MemoryListener};

/* waveform descriptions from https://www.nesdev.org/wiki/APU_Pulse */
const PULSE_DUTIES : [[bool;8];4] = [
    [false,true,false,false,false,false,false,false],
    [false,true,true,false,false,false,false,false],
    [false,true,true,true,true,false,false,false],
    [true,false,false,true,true,true,true,true],
];

const LENGTH_COUNTER_LOOKUP : [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6,
    160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30
];

pub struct Pulse
{
    first_address : u16,
    duty: usize,
    duty_index: usize,
    timer_period: u16,
    timer: u16,
    envelope: u8,
    divider: u8,
    decay_level: u8,
    start_flag: bool,
    constant_volume: bool,
    lc: u8,
    lc_halt: bool,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_divider: u8,
}

impl Pulse
{
    /* initializes a pulse, links it up as a listener on CoreMemory, and
     * wraps it appropriately
     */
    pub fn initialize(first_addr: u16, memory: &Rc<RefCell<CoreMemory>>) -> Rc<RefCell<Pulse>> {
        let pulse_ref = Rc::new(RefCell::new(Pulse::new(first_addr)));
        memory.borrow_mut().register_listener(pulse_ref.clone());

        pulse_ref
    }

    /* private constructor */
    fn new(first_address: u16) -> Pulse {
        Pulse {
            first_address,
            duty: 0,
            duty_index: 0,
            timer_period: 0,
            timer: 0,
            envelope: 0,
            divider: 0,
            decay_level: 0,
            start_flag: false,
            constant_volume: true,
            lc: 0,
            lc_halt: false,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
        }
    }

    pub(crate) fn tick(&mut self, apu_counter: u16) {
        /* on every tick: decrease timer; loop around at 0 */
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.duty_index = (self.duty_index + 1) % 8;
        } else {
            self.timer -= 1;
        }

        if
        (apu_counter == 3728 || apu_counter == 7456 || apu_counter == 11185 || apu_counter == 14914) {
            if self.start_flag {
                self.start_flag = false;
                self.decay_level = 15;
                self.divider = self.envelope;
            } else if self.divider == 0 {
                self.divider = self.envelope;
                if(self.decay_level == 0 && !self.lc_halt /* halt flag is also loop flag */) {
                    self.decay_level = 15;
                } else if self.decay_level > 0 {
                    self.decay_level -= 1;
                }
            } else {
                self.divider -= 1;
            }

            if apu_counter == 7456 || apu_counter == 14914 && !self.lc_halt && self.lc != 0 {
                self.lc -= 1;
                if self.sweep_enabled && self.sweep_shift != 0 && self.sweep_divider == 0 {
                    let change_amount = self.timer_period >> self.sweep_shift;
                    /* TODO: handle pulse 1 vs pulse 2 differences */
                    let target_period = if self.sweep_negate {
                        self.timer_period.saturating_sub(change_amount)
                    } else {
                        self.timer_period.saturating_add(change_amount)
                    };
                    self.timer_period = target_period;
                }
                if self.sweep_divider == 0 || self.sweep_reload {
                    self.sweep_divider = self.sweep_period;
                    self.sweep_reload = false;
                } else {
                    self.sweep_divider -= 1;
                }
            }
        }
    }

    fn set_duty_envelope(&mut self, byte0:u8) {
        /* duty info is stored in the first two bits; this waveform map is from the wiki */
        self.envelope = byte0 & 0xf;
        self.constant_volume = byte0 & 0x10 != 0;
        self.duty = ((byte0 & 0xc0) >> 6) as usize; /* NB: does not change duty_index */
        self.lc_halt = byte0 & 0x20 != 0;
        self.start_flag = true;
    }

    fn set_sweep(&mut self, byte1:u8) {
        self.sweep_enabled = byte1 & 0x80 == 0;
        self.sweep_period = (byte1 >> 4) & 0x7;
        self.sweep_negate = byte1 & 0x08 != 0;
        self.sweep_shift = byte1 & 0x07;
        self.sweep_reload = true;
        self.sweep_divider = self.sweep_period;
        /* TODO */
    }

    fn set_timer_lo(&mut self, byte2:u8) {
        self.timer_period = (self.timer_period & 0xff00) | (byte2 as u16);
    }

    fn set_lc_timer_hi(&mut self, byte3:u8) {
        self.timer_period = (self.timer_period & 0xff) | (((byte3 as u16) & 0x7) << 8);
        self.lc = LENGTH_COUNTER_LOOKUP[(byte3 >> 3) as usize];

        /* side-efects on write */
        self.duty_index = 0;
        self.start_flag = true;
    }

    fn should_play(&self) -> bool {
        self.lc != 0 && self.timer_period >= 8 && !self.lc_halt
    }

    pub fn amplitude(&self) -> f32 {
        if self.should_play() && PULSE_DUTIES[self.duty][self.duty_index] {
            (if self.constant_volume { self.envelope } else { self.decay_level }) as f32 / 15.0
        } else {
            0.0
        }
    }
}

impl MemoryListener for Pulse {
    fn get_addresses(&self) -> Vec<u16> {
        let mut addresses = Vec::new();

        addresses.push(self.first_address);
        addresses.push(self.first_address+1);
        addresses.push(self.first_address+2);
        addresses.push(self.first_address+3);

        addresses
    }

    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8 {
        /* open bus, this shouldn't be done */
        memory.read_no_listen(address)
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        match address - self.first_address {
            0 => {
                self.set_duty_envelope(value);
            }
            1 => {
                self.set_sweep(value);
            }
            2 => {
                self.set_timer_lo(value);
            }
            3 => {
                self.set_lc_timer_hi(value);
            }
            _ => {
                panic!("APU instrument passed invalid memory address 0x{:x}", address);
            }
        }
    }
}