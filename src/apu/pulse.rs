
/* waveform descriptions from https://www.nesdev.org/wiki/APU_Pulse */
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use rodio::{ChannelCount, SampleRate, Sink, Source};
use crate::cpu::{CoreMemory, MemoryListener};

const PULSE_DUTIES : [[bool;8];4] = [
    [false,true,false,false,false,false,false,false],
    [false,true,true,false,false,false,false,false],
    [false,true,true,true,true,false,false,false],
    [true,false,false,true,true,true,true,true],
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
    pub(crate) sink: Sink,
    queue: VecDeque<f32>,
}

impl Pulse
{
    pub(crate) fn from_addrs(first_address: u16, sink: Sink) -> Pulse {
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
            sink,
            queue: VecDeque::new(),
        }
    }

    pub(crate) fn tick(&mut self, apu_counter: u16) {
        /* TODO find a better way to sync this up */
        if apu_counter % 20/*(1790000/2/44100)*/ as u16 == 0 && self.queue.len() < 100000 {
            self.queue.push_back(self.amplitude());
        }

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
        self.lc = lc_lookup(byte3 >> 3);

        /* side-efects on write */
        self.duty_index = 0;
        self.start_flag = true;
    }

    fn should_play(&self) -> bool {
        self.lc != 0 && self.timer_period >= 8 && !self.lc_halt
    }

    fn amplitude(&self) -> f32 {
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

fn lc_lookup(val: u8) -> u8 {
    match val {
        /* linear lengths */
        0b11111 => 30,
        0b11101 => 28,
        0b11011 => 26,
        0b11001 => 24,
        0b10111 => 22,
        0b10101 => 20,
        0b10011 => 18,
        0b10001 => 16,
        0b01111 => 14,
        0b01101 => 12,
        0b01011 => 10,
        0b01001 => 8,
        0b00111 => 6,
        0b00101 => 4,
        0b00011 => 2,
        0b00001 => 254,

        /* base length 12, 4/4 at 75 bpm */
        0b11110 => 32,
        0b11100 => 16,
        0b11010 => 72,
        0b11000 => 192,
        0b10110 => 96,
        0b10100 => 48,
        0b10010 => 24,
        0b10000 => 12,

        /* Notes with base length 10 (4/4 at 90 bpm) */
        0b01110 => 26,
        0b01100 => 14,
        0b01010 => 60,
        0b01000 => 160,
        0b00110 => 80,
        0b00100 => 40,
        0b00010 => 20,
        0b00000 => 10,
        _ => panic!("unsupported LC lookup value"),
    }
}

pub struct PulseSource {
    pulse: Arc<RwLock<Pulse>>,
}

impl PulseSource {
    pub fn new(pulse: Arc<RwLock<Pulse>>) -> PulseSource {
        PulseSource {
            pulse,
        }
    }
}

impl Iterator for PulseSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        /* while in theory this allows for None, in practice the Sink will stop consuming new
         * events if next() returns None, so return something else to give the source time to
         * produce more samples
         */
        self.pulse.write().unwrap().queue.pop_front().or(Some(0.0))
    }
}

impl Source for PulseSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        1
    }

    fn sample_rate(&self) -> SampleRate {
        /* This is a little bit faster than the theoretical rate that we should be sampling at,
         * but it seems to be the best rate for keeping the sample queue from backing up;
         * not ideal, but seems to have th fewest issues overall
         */
        44800
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}