use crate::apu::APU;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::simulator::scheduler::TaskType::*;
use crate::simulator::SimulatorSignal;
use std::cell::RefCell;
use std::ops::Add;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
#[derive(Clone)]
enum TaskType {
    CPU,
    PPU,
    APU,
}

pub struct Scheduler {
    cpu: Box<CPU>,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
    receiver: Receiver<SimulatorSignal>,

    next_cpu_time: u64,
    next_ppu_time: u64,
    next_apu_time: u64,
}

impl Scheduler {
    pub fn new(
        cpu: Box<CPU>,
        ppu: Rc<RefCell<PPU>>,
        apu: Rc<RefCell<APU>>,
        receiver: Receiver<SimulatorSignal>,
    ) -> Self {
        Scheduler {
            cpu,
            ppu,
            apu,
            receiver,
            next_cpu_time: 0,
            next_ppu_time: 0,
            next_apu_time: 0,
        }
    }
    pub fn simulate(&mut self) {
        let start_time = Instant::now();

        let quantum = Duration::from_millis(10);

        let mut most_recent_now = start_time;
        let mut check_time_clocks = duration_to_clocks(quantum);

        loop {
            /* if we've received a message, terminate to prepare for joining */
            if let Ok(signal) = self.receiver.try_recv() {
                match signal {
                    /* handle save: write memory to save file */
                    SimulatorSignal::HandleSave(sx) => {
                        let _ = sx.send(self.cpu.get_save_data());
                    }
                    /* end simulation: break the loop */
                    SimulatorSignal::EndSimulation => {
                        return;
                    }
                }
            }

            let next_task = self.next_task();
            if next_task.1 > check_time_clocks {
                thread::sleep(
                    clocks_to_time(start_time, next_task.1)
                        .saturating_duration_since(most_recent_now),
                );
                most_recent_now = Instant::now();
                check_time_clocks =
                    duration_to_clocks(most_recent_now.add(quantum).duration_since(start_time));
            }

            match next_task {
                (CPU, time) => self.next_cpu_time = time + (self.cpu.transition() as u64) * 12,
                (PPU, time) => {
                    self.ppu.borrow_mut().tick(&mut self.cpu);
                    self.next_ppu_time = time + 4;
                }
                (APU, time) => {
                    let mut apu = self.apu.borrow_mut();
                    apu.apu_tick();
                    self.next_apu_time = time + 1 * 24;
                }
            }
        }
    }

    fn next_task(&self) -> (TaskType, u64) {
        let mut best_time = self.next_cpu_time;
        let mut best = CPU;

        if best_time > self.next_ppu_time {
            best_time = self.next_ppu_time;
            best = PPU;
        }

        if best_time > self.next_apu_time {
            best_time = self.next_apu_time;
            best = APU;
        }

        (best, best_time)
    }
}
/**
 * Given a starting time and a number of master clock ticks, returns the time at which that many
 * master clock ticks have passed since the start time. A master clock tick is defined, for an
 * NTSC system, as 1/4 of a PPU dot. TODO remove constants
 */
fn clocks_to_time(start_time: Instant, clocks: u64) -> Instant {
    start_time.add(Duration::from_micros(clocks * 1_000_000 / 21_477_272))
}

/**
 * Given a duration, converts it into "master clocks", which are defined for NTSC as 1/4 of a PPU
 * dot. This should be understood as a duration of master clock ticks, not a point in time.
 * TODO remove constants
 */
fn duration_to_clocks(duration: Duration) -> u64 {
    duration.as_micros() as u64 * 21_477_272 / 1_000_000
}
