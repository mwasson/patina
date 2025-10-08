use crate::apu::APU;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::processor::Processor;
use crate::scheduler::TaskType::*;
use std::cell::RefCell;
use std::ops::Add;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use winit::window::Window;

enum TaskType {
    CPU,
    PPU,
    APU,
}

pub(crate) fn simulate(
    cpu: &mut CPU,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
) {
    let start_time = Instant::now();
    let mut next_cpu_task = (CPU, start_time);
    let mut next_ppu_task = (PPU, start_time);
    let mut next_apu_task = (APU, start_time);

    let quantum = Duration::from_millis(10);

    let mut most_recent_now = start_time;
    let mut check_time = start_time.add(quantum);

    loop {
        let next_task = next_task(&next_cpu_task, &next_ppu_task, &next_apu_task);
        if next_task.1 > check_time {
            thread::sleep(next_task.1.saturating_duration_since(most_recent_now));
            most_recent_now = Instant::now();
            check_time = most_recent_now.add(quantum);
        }

        match next_task {
            (CPU, time) => {
                next_cpu_task = (CPU, cpu.transition(*time));
            }
            (PPU, time) => {
                let mut ppu = ppu.borrow_mut();
                ppu.tick();
                next_ppu_task = (PPU, time.add(ppu.cycles_to_duration(1)))
            }
            (APU, time) => {
                let mut apu = apu.borrow_mut();
                apu.apu_tick();
                next_apu_task = (APU, time.add(apu.cycles_to_duration(1)));
            }
        }
    }
}

fn next_task<'a>(
    t1: &'a (TaskType, Instant),
    t2: &'a (TaskType, Instant),
    t3: &'a (TaskType, Instant),
) -> &'a (TaskType, Instant) {
    let mut best = t1;

    if best.1.gt(&t2.1) {
        best = t2;
    }

    if best.1.gt(&t3.1) {
        best = t3;
    }

    best
}

pub struct RenderRequester {
    window: Option<Arc<Window>>,
}

impl RenderRequester {
    pub fn new() -> RenderRequester {
        RenderRequester { window: None }
    }
    pub fn set_window(&mut self, window: Arc<Window>) {
        self.window = Some(window);
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
