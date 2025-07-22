pub trait Processor {
    fn clock_speed(&self) -> u64;

    fn run_timed<F,U>(&self, cycles:u64, f: F) -> U where
        F: FnOnce() -> U
    {
        let start_time = std::time::Instant::now();
        let result = f();
        hertz::sleep_for_constant_rate((self.clock_speed()/cycles) as usize, start_time);

        result
    }
}