use crate::apu::dmc::DMC;
use crate::apu::noise::Noise;
use crate::apu::pulse::Pulse;
use crate::apu::triangle::Triangle;
use crate::cpu::{CoreMemory, MemoryListener};
use crate::processor::Processor;
use rodio::{ChannelCount, OutputStream, SampleRate, Sink, Source};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
/* TODO This is a little bit faster than the theoretical rate that we should be sampling at,
 * but it seems to be the best rate for keeping the sample queue from backing up;
 * not ideal, but seems to have the fewest issues overall
 */

pub struct APU {
    apu_counter: u16,
    _output_stream: OutputStream, /* can't remove this--if it's collected, sound won't play */
    _sink: Sink,                  /* ditto--confusingly, since OutputStream should have a ref */
    pulse1: Pulse,                /* to it through the Mixer? */
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: DMC,
    status: u8,
    queue: Arc<RwLock<VecDeque<f32>>>,
}

const PULSE_1_FIRST_ADDR: u16 = 0x4000;
const PULSE_2_FIRST_ADDR: u16 = 0x4004;

impl APU {
    pub fn new() -> Rc<RefCell<APU>> {
        let stream_handle =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = Sink::connect_new(&stream_handle.mixer());
        let queue = Arc::new(RwLock::new(VecDeque::new()));
        sink.append(BufferedMixedSource::new(queue.clone()));

        let pulse1 = Pulse::new(PULSE_1_FIRST_ADDR, true);
        let pulse2 = Pulse::new(PULSE_2_FIRST_ADDR, false);
        let triangle = Triangle::new();
        let noise = Noise::new();
        let dmc = DMC::new();

        Rc::new(RefCell::new(APU {
            apu_counter: 0,
            _output_stream: stream_handle,
            pulse1,
            pulse2,
            triangle,
            noise,
            dmc,
            queue,
            _sink: sink,
            status: 0,
        }))
    }

    pub fn apu_tick(&mut self) {
        self.apu_counter = (self.apu_counter + 1) % 14915;

        self.pulse1.tick(self.apu_counter);
        self.pulse2.tick(self.apu_counter);
        self.triangle.tick(self.apu_counter);
        self.noise.tick(self.apu_counter);
        self.dmc.tick(self.apu_counter);

        /* TODO find a better way to sync this up */
        if self.apu_counter % 20 == 0 /* TODO */ && self.queue.read().unwrap().len() < 50000 {
            self.queue.write().unwrap().push_back(self.mix());
        }
    }

    fn mix(&self) -> f32 {
        let pulse1_vol = self.pulse1.amplitude();
        let pulse2_vol = self.pulse2.amplitude();
        let triangle_vol = self.triangle.amplitude();
        let noise_vol = self.noise.amplitude();
        let dmc_vol = self.dmc.amplitude();

        /* formulae from https://www.nesdev.org/wiki/APU_Mixer */
        let pulse_out = 95.88 / (8128.0 / (pulse1_vol + pulse2_vol) + 100.0);
        let tnd_out = 159.79
            / (1.0 / (triangle_vol / 8227.0 + noise_vol / 12241.0 + dmc_vol / 22638.0) + 100.0);

        pulse_out + tnd_out
    }

    #[cfg(test)]
    pub fn get_status(&self) -> u8 {
        self.status
    }
}

impl Processor for APU {
    fn clock_speed(&self) -> u64 {
        894880 //1_789_773/2 /* TODO constantize */
    }
}

impl MemoryListener for APU {
    fn get_addresses(&self) -> Vec<u16> {
        [
            0x4000, 0x4001, 0x4002, 0x4003, /* pulse 1 */
            0x4004, 0x4005, 0x4006, 0x4007, /* pulse 2 */
            0x4008, 0x4009, 0x400a, 0x400b, /* triangle */
            0x400c, 0x400d, 0x400e, 0x400f, /* noise */
            0x4010, 0x4011, 0x4012, 0x4013, /* dmc */
            0x4015, 0x4017,
        ]
        .to_vec() /* apu control regs */
    }

    fn read(&mut self, memory: &CoreMemory, _address: u16) -> u8 {
        memory.open_bus()
    }

    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8) {
        match address & 0x001c {
            /* pulse 1:  xxx0 00xx */
            0x00 => self.pulse1.write(memory, address, value),
            /* pulse 2:  xxx0 01xx */
            0x04 => self.pulse2.write(memory, address, value),
            /* triangle: xxx0 10xx */
            0x08 => self.triangle.write(memory, address, value),
            /* noise:    xxx0 11xx */
            0x0c => self.noise.write(memory, address, value),
            /* dmc:      xxx1 00xx */
            0x10 => self.dmc.write(memory, address, value),
            _ => {
                if address == 0x4015 {
                    self.status = value;
                    self.pulse1.set_enabled(value & 0x1 != 0);
                    self.pulse2.set_enabled(value & 0x2 != 0);
                    self.triangle.set_enabled(value & 0x4 != 0);
                    self.noise.set_enabled(value & 0x8 != 0);
                    self.dmc.set_enabled(value & 0x10 != 0);
                }
            }
        }
    }
}

impl BufferedMixedSource {
    fn new(queue: Arc<RwLock<VecDeque<f32>>>) -> BufferedMixedSource {
        BufferedMixedSource { queue }
    }
}

pub struct BufferedMixedSource {
    queue: Arc<RwLock<VecDeque<f32>>>,
}

impl Iterator for BufferedMixedSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        /* while in theory this allows for None, in practice the Sink will stop consuming new
         * events if next() returns None, so return something else to give the source time to
         * produce more samples
         */
        self.queue.write().unwrap().pop_front().or(Some(0.0))
    }
}

impl Source for BufferedMixedSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        1
    }

    fn sample_rate(&self) -> SampleRate {
        44744
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
