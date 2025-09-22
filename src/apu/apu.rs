use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use rodio::{Sink, Source};
use rodio::source::{SignalGenerator, Stoppable, TakeDuration};
use crate::cpu::{CoreMemory, MemoryListener};
use crate::processor::Processor;

pub struct APU {
    apu_counter: u8,
    pulse1: Rc<RefCell<Pulse>>,
    pulse2: Rc<RefCell<Pulse>>,
}

const FREQ_CPU : f32 = 1_789_773f32; /* NB: NTSC only, different for PAL */

const PULSE_1_FIRST_ADDR : u16 = 0x4000;
const PULSE_2_FIRST_ADDR : u16 = 0x4004;

impl APU {
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> APU {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");

        let pulse1 = Rc::new(RefCell::new(Pulse::from_addrs(PULSE_1_FIRST_ADDR, Sink::connect_new(&stream_handle.mixer()))));
        memory.borrow_mut().register_listener(pulse1.clone());

        let pulse2 = Rc::new(RefCell::new(Pulse::from_addrs(PULSE_2_FIRST_ADDR, Sink::connect_new(&stream_handle.mixer()))));
        memory.borrow_mut().register_listener(pulse2.clone());

        APU {
            apu_counter: 0,
            pulse1,
            pulse2,
        }
    }
    pub fn apu_tick(&mut self) {
        self.apu_counter += 1;

        if self.apu_counter % 2 == 0 {
            self.pulse1.borrow_mut().check();
            self.pulse2.borrow_mut().check();
        }
    }
}

impl Processor for APU {
    fn clock_speed(&self) -> u64 {
        1790000/2 /* TODO constantize */
    }
}

/* waveform descriptions from https://www.nesdev.org/wiki/APU_Pulse
 * in that table low is 0 and high is 1, but Rodio expects values from -1 to 1 */
const PULSE_DUTY_0 : [f32;8] = [-1.0,1.0,-1.0,-1.0,-1.0,-1.0,-1.0,-1.0];
const PULSE_DUTY_1 : [f32;8] = [-1.0,1.0,1.0,-1.0,-1.0,-1.0,-1.0,-1.0];
const PULSE_DUTY_2 : [f32;8] = [-1.0,1.0,1.0,1.0,1.0,-1.0,-1.0,-1.0];
const PULSE_DUTY_3 : [f32;8] = [1.0,-1.0,-1.0,1.0,1.0,1.0,1.0,1.0];

struct Pulse
{
    first_address : u16,
    duty: fn(f32) -> f32,
    timer: u16,
    lc: u8,
    lc_halt: bool,
    sink: Sink,
    source: Option<Stoppable<TakeDuration<SignalGenerator>>>,
}

impl Pulse
{
    fn from_addrs(first_address: u16, sink: Sink) -> Pulse {
        /* TODO remove */
        Pulse {
            first_address,
            duty: pulse_duty_func_0,
            timer: 0,
            lc: 0,
            lc_halt: false,
            sink,
            source: None,
        }
    }

    fn check(&mut self) {
        if !self.lc_halt && self.lc != 0 {
            self.lc -= 1;

            if self.lc == 0 {
                self.sink.stop();
            }
        }
    }

    fn freq(&self) -> f32 {
        FREQ_CPU / (16 * (self.timer + 1)) as f32
    }

    fn get_source(&self) -> Stoppable<TakeDuration<SignalGenerator>> {
        SignalGenerator::with_function(44100 /* TODO */, self.freq(), self.duty)
            .take_duration(Duration::from_secs(100))
            .stoppable()
    }

    fn set_duty(&mut self, byte0:u8) {
        /* duty info is stored in the first two bits; this waveform map is from the wiki */
        let duty_val = (byte0 & 0xc0) >> 6;
        assert!(duty_val <= 3);
        self.duty = match duty_val {
            0 => pulse_duty_func_0,
            1 => pulse_duty_func_1,
            2 => pulse_duty_func_2,
            3 => pulse_duty_func_3,
            _ => panic!("impossible duty value"),
        };
        self.lc_halt = byte0 & 0x20 != 0;
        /* TODO other bits */
    }

    fn set_sweep(&mut self, byte1:u8) {
        /* TODO */
    }

    fn set_timer_lo(&mut self, byte2:u8) {
        self.timer = (self.timer & 0xff00) | (byte2 as u16);
    }

    fn set_lc_timer_hi(&mut self, byte3:u8) {
        self.timer = (self.timer & 0xff) | (((byte3 as u16) & 0x7) << 8);
        self.lc = lc_lookup(byte3 >> 3);
        if self.lc != 0 {
            self.source = Some(self.get_source());
            self.sink.append(self.get_source());
        }
    }
}

fn pulse_duty_general(time: f32, duty_array:&[f32;8]) -> f32 {
    /* TODO does the wrong thing at 1.0 */
    let index = (time.fract() * 8.0).floor() as usize;
    duty_array[index]
}

fn pulse_duty_func_0(time: f32) -> f32 {
    pulse_duty_general(time, &PULSE_DUTY_0)
}

fn pulse_duty_func_1(time: f32) -> f32 {
    pulse_duty_general(time, &PULSE_DUTY_1)
}

fn pulse_duty_func_2(time: f32) -> f32 {
    pulse_duty_general(time, &PULSE_DUTY_2)
}

fn pulse_duty_func_3(time: f32) -> f32 {
    pulse_duty_general(time, &PULSE_DUTY_3)
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
                self.set_duty(value);
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
