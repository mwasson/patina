use std::cmp::Reverse;
use std::hash::{Hash};
use std::ops::Add;
use std::thread;
use std::time::Instant;
use priority_queue::PriorityQueue;
use crate::apu::APU;
use crate::cpu::{CPU};
use crate::ppu::PPU;
use crate::processor::Processor;
use crate::scheduler::TaskType::*;

#[derive(Hash,Eq,PartialEq)]
enum TaskType
{
    CPU,
    PPUScreen,
    PPUScanline(u8),
    PPUVBlank,
    APU,
}

pub(crate) fn simulate(cpu: &mut CPU, ppu: &mut PPU, apu: &mut APU) {
    let mut tasks : PriorityQueue<TaskType,Reverse<Instant>>  = PriorityQueue::new();

    let rev_start_time = Reverse(Instant::now());
    tasks.push(CPU, rev_start_time);
    tasks.push(PPUScreen, rev_start_time);
    tasks.push(APU, rev_start_time);

    loop {
        match tasks.pop() {
            Some((task, rev_time)) => {
                let time = rev_time.0;
                let sleep_time = time.saturating_duration_since(Instant::now());
                if !sleep_time.is_zero() {
                    thread::sleep(sleep_time);
                }

                match task {
                    CPU => {
                        let next_time = cpu.transition(time);
                        tasks.push(CPU, Reverse(next_time));
                    },
                    PPUScreen => {
                        ppu.beginning_of_screen_render();
                        let scanline_duration = ppu.cycles_to_duration(341);
                        let mut scanline_time = time;
                        for i in 0..240 {
                            /* first scanline is in 341 cycles */
                            scanline_time = scanline_time.add(scanline_duration);
                            tasks.push(PPUScanline(i as u8), Reverse(scanline_time));
                        }
                        tasks.push(PPUVBlank, Reverse(scanline_time.add(scanline_duration)));
                    },
                    PPUScanline(scanline) => {
                        ppu.render_scanline(scanline)
                    },
                    PPUVBlank => {
                        ppu.end_of_screen_render();
                        tasks.push(PPUScreen, Reverse(time.add(ppu.cycles_to_duration(21*341))));
                    }
                    APU => {
                        apu.apu_tick();
                        tasks.push(APU, Reverse(time.add(apu.cycles_to_duration(1))));
                    }
                }
            }
            _ => {
                panic!("Nothing in the task queue?!");
            }
        }
    }
}