use crate::apu::APU;
use crate::cpu::CPU;
use crate::ppu::PPU;
use crate::scheduler::TaskType::*;
use std::cell::RefCell;
use std::ops::Add;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use winit::window::Window;

enum TaskType {
    CPU,
    PPUScreen,
    PPUScanline(u8, u8),
    PPUVBlank,
    APU,
}

pub(crate) fn simulate(
    cpu: &mut CPU,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
    requester: Arc<Mutex<RenderRequester>>,
) {
    let start_time = Instant::now();
    let mut next_cpu_task = (CPU, 0);
    let mut next_ppu_task = (PPUScreen, 0);
    let mut next_apu_task = (APU, 0);

    let quantum = Duration::from_millis(10);

    let mut most_recent_now = start_time;
    let mut check_time_clocks = duration_to_clocks(quantum);

    loop {
        let next_task = next_task(&next_cpu_task, &next_ppu_task, &next_apu_task);
        if next_task.1 > check_time_clocks {
            thread::sleep(
                clocks_to_time(start_time, next_task.1).saturating_duration_since(most_recent_now),
            );
            most_recent_now = Instant::now();
            check_time_clocks =
                duration_to_clocks(most_recent_now.add(quantum).duration_since(start_time));
        }

        match next_task {
            (CPU, time) => {
                next_cpu_task = (CPU, time + (cpu.transition() as u64) * 12);
            }
            (PPUScreen, time) => {
                let mut borrowed_ppu = ppu.borrow_mut();

                borrowed_ppu.beginning_of_screen_render();

                next_ppu_task = (PPUScanline(0, 0), time + 2 * 4)
            }
            (PPUScanline(scanline_ref, x_ref), time) => {
                let scanline = *scanline_ref;
                let x = *x_ref;
                let mut borrowed_ppu = ppu.borrow_mut();

                if x == 0 {
                    borrowed_ppu.render_scanline_begin(scanline);
                }

                borrowed_ppu.render_pixel(scanline, x);

                if x == 0xff {
                    borrowed_ppu.render_scanline_end();
                }

                let (next_task_type, lol_rename) = if x == 0xff {
                    if scanline == 239 {
                        (PPUVBlank, time + 84 * 4)
                    } else {
                        (PPUScanline(scanline + 1, 0), time + (84 + 1) * 4)
                    }
                } else {
                    (PPUScanline(scanline, x + 1), time + 1 * 4)
                };
                next_ppu_task = (next_task_type, lol_rename)
            }
            (PPUVBlank, time) => {
                let mut borrowed_ppu = ppu.borrow_mut();
                borrowed_ppu.end_of_screen_render(cpu);

                /* send window message to redraw */
                requester.lock().unwrap().request_redraw();

                next_ppu_task = (PPUScreen, time + (21 * 341 + 304) * 4);
            }
            (APU, time) => {
                let mut apu = apu.borrow_mut();
                apu.apu_tick();
                next_apu_task = (APU, time + 1 * 24);
            }
        }
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

fn next_task<'a>(
    t1: &'a (TaskType, u64),
    t2: &'a (TaskType, u64),
    t3: &'a (TaskType, u64),
) -> &'a (TaskType, u64) {
    let mut best = t1;

    if best.1 > t2.1 {
        best = t2;
    }

    if best.1 > t3.1 {
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
