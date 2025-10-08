use crate::apu::APU;
use crate::cpu::{SharedItems, CPU};
use crate::ppu::PPU;
use crate::processor::Processor;
use crate::scheduler::TaskType::*;
use std::cell::RefCell;
use std::ops::Add;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use winit::window::Window;
use crate::mapper::Mapper;

enum TaskType {
    CPU,
    PPUScreen,
    PPUScanline(u8, u8),
    PPUVBlank,
    APU,
}

pub(crate) fn simulate(
    cpu: &mut CPU,
    ppu: &mut PPU,
    apu: &mut APU,
    mapper: &mut dyn Mapper,
    requester: Arc<Mutex<RenderRequester>>,
) {
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
        
        let mut shared_items = SharedItems {
            apu,
            ppu,
            mapper,
        };

        match next_task {
            (CPU, time) => {
                let next_time = cpu.transition(&mut shared_items, *time);
                next_cpu_task = (CPU, next_time);
            }
            (PPUScreen, time) => {
                ppu.beginning_of_screen_render();

                let scanline_duration = ppu.cycles_to_duration(341 - 340 + 1);
                next_ppu_task = (PPUScanline(0, 0), time.add(scanline_duration))
            }
            (PPUScanline(scanline_ref, x_ref), time) => {
                let scanline = *scanline_ref;
                let x = *x_ref;

                if x == 0 {
                    ppu.render_scanline_begin(mapper, scanline);
                }

                ppu.render_pixel(mapper, scanline, x);

                if x == 0xff {
                    ppu.render_scanline_end();
                }

                let (next_task_type, cycles_to_wait) = if x == 0xff {
                    if scanline == 239 {
                        (PPUVBlank, 84)
                    } else {
                        (PPUScanline(scanline + 1, 0), 84 + 1)
                    }
                } else {
                    (PPUScanline(scanline, x + 1), 1)
                };
                let next_time = time.add(ppu.cycles_to_duration(cycles_to_wait));
                next_ppu_task = (next_task_type, next_time)
            }
            (PPUVBlank, time) => {
                ppu.end_of_screen_render(cpu);

                /* send window message to redraw */
                requester.lock().unwrap().request_redraw();

                next_ppu_task = (
                    PPUScreen,
                    time.add(ppu.cycles_to_duration(21 * 341 + 304)),
                );
            }
            (APU, time) => {
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
