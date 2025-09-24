
const LENGTH_COUNTER_LOOKUP : [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6,
    160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30
];

pub struct LengthCounter {
    period: u8,
    count: u8,
    halt: bool,
}

impl LengthCounter {
    pub fn new() -> LengthCounter {
        LengthCounter {
            period: 0,
            count: 0,
            halt: false,
        }
    }

    pub fn clock(&mut self) {
        if !self.halt && self.count != 0 {
            self.count -= 1;
        }
    }

    pub fn amplitude(&self) -> f32 {
        if self.count > 0 { 1.0 } else { 0.0 }
    }

    pub fn set_halt(&mut self, halt: bool) {
        self.halt = halt;
    }

    pub fn set_lc(&mut self, data: u8) {
        self.period = LENGTH_COUNTER_LOOKUP[(data >> 3) as usize];
        self.count = self.period;
    }
}