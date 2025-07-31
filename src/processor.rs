use std::any::Any;
use std::time;
use std::time::Instant;

pub trait Processor {
    fn clock_speed(&self) -> u64;

    fn run_timed<F,U>(&mut self, cycles:u32, f: F) -> U where
        F: FnOnce(&mut Self) -> U,
    {
        self.run_time_with_start(Instant::now(), cycles, f)
    }

    fn run_time_with_start<F,U>(&mut self, start_time: Instant, cycles:u32, f: F) -> U where
        F: FnOnce(&mut Self) -> U,
    {
        let result = f(self);
        let ns = (1e9 as u64)*(cycles as u64)/(self.clock_speed());
        let frame_duration = time::Duration::from_nanos(ns);
        if(start_time.elapsed() > 3*frame_duration) {
            println!("YOOO TOOK AT LEAST THREE TIMES LONGER THAN EXPECTED ({} cycles): {}x)", cycles, start_time.elapsed().as_nanos()/frame_duration.as_nanos());
        }
        std::thread::sleep(frame_duration.saturating_sub(start_time.elapsed()));

        result
    }
}