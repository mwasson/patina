use std::cell::RefCell;
use std::hash::Hash;
use std::ops::Add;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use winit::window::Window;
use crate::apu::APU;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::processor::Processor;
use crate::scheduler::TaskType::*;

#[derive(Hash,Eq,PartialEq,Clone)]
enum TaskType
{
    CPU,
    PPUScreen,
    PPUScanline(u8),
    PPUVBlank,
    APU,
}

#[inline(never)]
pub(crate) fn simulate(cpu: &mut CPU, ppu: Rc<RefCell<PPU>>, apu: Rc<RefCell<APU>>, requester: Arc<Mutex<RenderRequester>>) {
    let start_time = Instant::now();
    let mut next_cpu_task = (CPU, start_time);
    let mut next_ppu_task = (PPUScreen, start_time);
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
                next_cpu_task = (CPU, cpu.transition(time));
            },
            (PPUScreen, time) => {
                let mut borrowed_ppu = ppu.borrow_mut();
                borrowed_ppu.beginning_of_screen_render();

                let scanline_duration = borrowed_ppu.cycles_to_duration(341);
                next_ppu_task = (PPUScanline(0), time.add(scanline_duration))
            },
            (PPUScanline(scanline), time) => {
                let mut borrowed_ppu = ppu.borrow_mut();
                borrowed_ppu.render_scanline(scanline);

                let scanline_duration = borrowed_ppu.cycles_to_duration(341);
                let next_task_type = if scanline == 239 {PPUVBlank} else { PPUScanline(scanline+1) };
                let next_time = time.add(scanline_duration);
                next_ppu_task = (next_task_type, next_time)
            },
            (PPUVBlank, time) => {
                let mut borrowed_ppu = ppu.borrow_mut();
                borrowed_ppu.end_of_screen_render();

                /* send window message to redraw */
                requester.lock().unwrap().request_redraw();
                
                next_ppu_task = (PPUScreen, time.add(borrowed_ppu.cycles_to_duration(21 * 341)));

            },
            (APU, time) => {
                let mut apu = apu.borrow_mut();
                apu.apu_tick();
                next_apu_task = (APU, time.add(apu.cycles_to_duration(1)));
            }
        }
    }
}

fn next_task(t1: &(TaskType,Instant), t2: &(TaskType,Instant), t3: &(TaskType,Instant)) -> (TaskType,Instant) {
    let mut best = t1;

    if best.1.gt(&t2.1) {
        best = t2;
    }

    if best.1.gt(&t3.1) {
        best = t3;
    }

    best.clone()
}

pub struct RenderRequester {
	window: Option<Arc<Window>>
}

impl RenderRequester {
	pub fn new() -> RenderRequester {
		RenderRequester {
			window: None
		}
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