use crate::apu::pulse::Pulse;
use crate::cpu::CoreMemory;
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
const SAMPLE_RATE : SampleRate = 44800;

pub struct APU {
    apu_counter: u16,
    output_stream: OutputStream, /* can't remove this--if it's collected, sound won't play */
    sink: Sink,                  /* ditto--confusingly, since OutputStream should have a ref */
    pulse1: Rc<RefCell<Pulse>>,  /* to it through the Mixer? */
    pulse2: Rc<RefCell<Pulse>>,
    queue: Arc<RwLock<VecDeque<f32>>>,
}

const FREQ_CPU : f32 = 1_789_773f32; /* NB: NTSC only, different for PAL */

const PULSE_1_FIRST_ADDR : u16 = 0x4000;
const PULSE_2_FIRST_ADDR : u16 = 0x4004;

impl APU {
    pub fn new(memory: Rc<RefCell<CoreMemory>>) -> APU {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");
        let sink = Sink::connect_new(&stream_handle.mixer());
        let queue = Arc::new(RwLock::new(VecDeque::new()));
        sink.append(BufferedMixedSource::new(queue.clone()));

        let pulse1: Rc<RefCell<Pulse>> = Pulse::initialize(PULSE_1_FIRST_ADDR, &memory);
        let pulse2: Rc<RefCell<Pulse>> = Pulse::initialize(PULSE_2_FIRST_ADDR, &memory);

        APU {
            apu_counter: 0,
            output_stream: stream_handle,
            pulse1,
            pulse2,
            queue,
            sink
        }
    }
    pub fn apu_tick(&mut self) {
        self.apu_counter += 1;

        if(self.apu_counter == 14915) {
            self.apu_counter = 0;
        }

        self.pulse1.borrow_mut().tick(self.apu_counter);
        self.pulse2.borrow_mut().tick(self.apu_counter);

        /* TODO sample and mix */

        /* TODO find a better way to sync this up */
        let mut queue = self.queue.write().unwrap();
        if self.apu_counter % 20 == 0 /* TODO */ && queue.len() < 3000 {
            queue.push_back(self.mix());
        }
    }

    fn mix(&self) -> f32 {
        let pulse1_vol = self.pulse1.borrow().amplitude();
        let pulse2_vol = self.pulse2.borrow().amplitude();

        /* formula from https://www.nesdev.org/wiki/APU_Mixer */
        let pulse_out = 95.88/(8128.0 / (pulse1_vol + pulse2_vol) + 100.0);

        pulse_out
    }
}

impl Processor for APU {
    fn clock_speed(&self) -> u64 {
        1_789_773/2 /* TODO constantize */
    }
}

impl BufferedMixedSource {
    fn new(queue: Arc<RwLock<VecDeque<f32>>>) -> BufferedMixedSource {
        BufferedMixedSource {
            queue,
        }
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
        44800
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}