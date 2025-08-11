use std::time;
use std::time::Instant;

pub trait Processor {
    fn clock_speed(&self) -> u64;

    fn run_timed<F,U>(&mut self, cycles:u128, f:F) -> U where
        F: FnOnce(&mut Self) -> U,
    {
        self.run_timed_from_start(cycles, Instant::now(), f)
    }

    fn run_timed_from_start<F,U>(&mut self, cycles:u128, start_time:Instant, f: F) -> U where
        F: FnOnce(&mut Self) -> U,
    {
        let result = f(self);
        let ns = (1e9 as u64)*(cycles as u64)/(self.clock_speed());
        let frame_duration = time::Duration::from_nanos(ns);

        std::thread::sleep(frame_duration.saturating_sub(start_time.elapsed()));

        result
    }
}