pub struct Timer {
    pub period: u16,
    pub count: u16,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            period: 0,
            count: 0,
        }
    }

    /* Returns true if the timer looped */
    pub fn clock(&mut self) -> bool{
        if self.count == 0 {
            self.count = self.period;
            true
        } else {
            self.count -= 1;
            false
        }
    }
    
    /* for setting the period directly */
    pub fn set_period(&mut self, period: u16) {
        self.period = period;
    }

    pub fn set_timer_lo(&mut self, data: u8) {
        self.period = (self.period & 0xff00) | (data as u16);
    }

    pub fn set_timer_hi(&mut self, data: u8) {
        self.period = (self.period & 0xff) | (((data as u16) & 0x7) << 8);
    }
}