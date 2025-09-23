use crate::apu::pulse::{Pulse, PulseSource};
use crate::cpu::{CoreMemory, MemoryListener};
use crate::processor::Processor;
use rodio::{OutputStream, Sink};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

pub struct APU {
    apu_counter: u16,
    memory: Rc<RefCell<CoreMemory>>,
    output_stream: OutputStream, /* TODO can't remove this */
    pulse1: Arc<RwLock<Pulse>>,
    pulse2: Arc<RwLock<Pulse>>,
}

const FREQ_CPU : f32 = 1_789_773f32; /* NB: NTSC only, different for PAL */

const PULSE_1_FIRST_ADDR : u16 = 0x4000;
const PULSE_2_FIRST_ADDR : u16 = 0x4004;

impl APU {
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> APU {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");

        let pulse1 = Arc::new(RwLock::new(Pulse::from_addrs(PULSE_1_FIRST_ADDR, Sink::connect_new(&stream_handle.mixer()))));
        memory.borrow_mut().register_listener(pulse1.clone());
        pulse1.write().unwrap().sink.append(PulseSource::new(pulse1.clone()));

        let pulse2 = Arc::new(RwLock::new(Pulse::from_addrs(PULSE_2_FIRST_ADDR, Sink::connect_new(&stream_handle.mixer()))));
        memory.borrow_mut().register_listener(pulse2.clone());
        pulse2.write().unwrap().sink.append(PulseSource::new(pulse2.clone()));

        APU {
            apu_counter: 0,
            memory: memory.clone(),
            output_stream: stream_handle,
            pulse1,
            pulse2,
        }
    }
    pub fn apu_tick(&mut self) {
        self.apu_counter += 1;

        if(self.apu_counter == 14915) {
            self.apu_counter = 0;
        }

        self.pulse1.write().unwrap().tick(self.apu_counter);
        self.pulse2.write().unwrap().tick(self.apu_counter);
    }
}

impl Processor for APU {
    fn clock_speed(&self) -> u64 {
        1_789_773/2 /* TODO constantize */
    }
}