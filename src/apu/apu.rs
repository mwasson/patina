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
    pulse1: Rc<RefCell<Pulse>>,   /* to it through the Mixer? */
    pulse2: Rc<RefCell<Pulse>>,
    triangle: Rc<RefCell<Triangle>>,
    noise: Rc<RefCell<Noise>>,
    dmc: Rc<RefCell<DMC>>,
    queue: Arc<RwLock<VecDeque<f32>>>,
}

const PULSE_1_FIRST_ADDR: u16 = 0x4000;
const PULSE_2_FIRST_ADDR: u16 = 0x4004;

impl APU {
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> Rc<RefCell<APU>> {
        let stream_handle =
            rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = Sink::connect_new(&stream_handle.mixer());
        let queue = Arc::new(RwLock::new(VecDeque::new()));
        sink.append(BufferedMixedSource::new(queue.clone()));

        let pulse1: Rc<RefCell<Pulse>> = Pulse::initialize(PULSE_1_FIRST_ADDR, true, &memory);
        let pulse2: Rc<RefCell<Pulse>> = Pulse::initialize(PULSE_2_FIRST_ADDR, false, &memory);
        let triangle: Rc<RefCell<Triangle>> = Triangle::initialize(&memory);
        let noise: Rc<RefCell<Noise>> = Noise::initialize(&memory);
        let dmc: Rc<RefCell<DMC>> = DMC::initialize(&memory);

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
        }))
    }

    pub fn apu_tick(&mut self) {
        self.apu_counter = (self.apu_counter + 1) % 14915;

        self.pulse1.borrow_mut().tick(self.apu_counter);
        self.pulse2.borrow_mut().tick(self.apu_counter);
        self.triangle.borrow_mut().tick(self.apu_counter);
        self.noise.borrow_mut().tick(self.apu_counter);
        self.dmc.borrow_mut().tick(self.apu_counter);

        /* TODO find a better way to sync this up */
        if self.apu_counter % 20 == 0 /* TODO */ && self.queue.read().unwrap().len() < 50000 {
            self.queue.write().unwrap().push_back(self.mix());
        }
    }

    fn mix(&self) -> f32 {
        let pulse1_vol = self.pulse1.borrow().amplitude();
        let pulse2_vol = self.pulse2.borrow().amplitude();
        let triangle_vol = self.triangle.borrow().amplitude();
        let noise_vol = self.noise.borrow().amplitude();
        let dmc_vol = self.dmc.borrow().amplitude();

        /* formulae from https://www.nesdev.org/wiki/APU_Mixer */
        let pulse_out = 95.88 / (8128.0 / (pulse1_vol + pulse2_vol) + 100.0);
        let tnd_out = 159.79
            / (1.0 / (triangle_vol / 8227.0 + noise_vol / 12241.0 + dmc_vol / 22638.0) + 100.0);

        pulse_out + tnd_out
    }
}

impl Processor for APU {
    fn clock_speed(&self) -> u64 {
        894880 //1_789_773/2 /* TODO constantize */
    }
}

impl MemoryListener for APU {
    fn get_addresses(&self) -> Vec<u16> {
        [0x4015, 0x4017].to_vec()
    }

    fn read(&mut self, memory: &CoreMemory, _address: u16) -> u8 {
        memory.open_bus()
    }

    fn write(&mut self, _memory: &CoreMemory, address: u16, value: u8) {
        if address == 0x4015 {
            self.pulse1.borrow_mut().set_enabled(value & 0x1 != 0);
            self.pulse2.borrow_mut().set_enabled(value & 0x2 != 0);
            self.triangle.borrow_mut().set_enabled(value & 0x4 != 0);
            self.noise.borrow_mut().set_enabled(value & 0x8 != 0);
            self.dmc.borrow_mut().set_enabled(value & 0x10 != 0);
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
