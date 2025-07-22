use std::time;

pub trait Processor {
    fn clock_speed(&self) -> u64;

    fn run_timed<F,U>(&mut self, cycles:u16, f: F) -> U where
        F: FnOnce(&mut Self) -> U,
    {
        let start_time = time::Instant::now();
        let result = f(self);
        let ns = (1e9 as u64)*(cycles as u64)/(self.clock_speed());
        let frame_duration = time::Duration::from_nanos(ns);
        std::thread::sleep(frame_duration.saturating_sub(start_time.elapsed()));

        result
    }
}