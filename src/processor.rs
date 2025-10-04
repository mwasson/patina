use std::time::{Duration, Instant};

pub trait Processor {
    fn clock_speed(&self) -> u64;

    fn cycles_to_duration(&self, cycles: u16) -> Duration {
        Duration::from_nanos((1e9 as u64) * (cycles as u64) / (self.clock_speed()))
    }

    fn _run_timed<F, U>(&mut self, cycles: u128, f: F) -> U
    where
        F: FnOnce(&mut Self) -> U,
    {
        self._run_timed_from_start(cycles, Instant::now(), f)
    }

    fn _run_timed_from_start<F, U>(&mut self, cycles: u128, start_time: Instant, f: F) -> U
    where
        F: FnOnce(&mut Self) -> U,
    {
        let result = f(self);
        let ns = (1e9 as u64) * (cycles as u64) / (self.clock_speed());
        let frame_duration = Duration::from_nanos(ns);
        let sleep_time = frame_duration.saturating_sub(start_time.elapsed());

        // while !frame_duration.saturating_sub(start_time.elapsed()).is_zero() {
        //
        // }

        if !sleep_time.is_zero() {
            std::thread::sleep(sleep_time);
        }
        result
    }
}
