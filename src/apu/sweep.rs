use crate::apu::timer::Timer;

pub struct Sweep {
    divider: u8,
    enabled: bool,
    negate: bool,
    period: u8,
    reload: bool,
    shift: u8,
    muting: bool,
    ones_complement: bool,
}

impl Sweep {
    pub fn new(ones_complement: bool) -> Sweep {
        Sweep {
            divider: 0,
            enabled: false,
            negate: false,
            period: 0,
            reload: false,
            shift: 0,
            muting: false,
            ones_complement,
        }
    }

    pub fn clock(&mut self, timer: &mut Timer) {
        self.muting = timer.period < 8;
        if self.enabled && self.shift != 0 && self.divider == 0 {
            self.muting = false;
            let mut change_amount = timer.period >> self.shift;
            let target_period = if self.negate {
                if self.ones_complement {
                    change_amount += 1;
                }
                timer.period.saturating_sub(change_amount)
            } else {
                timer.period.wrapping_add(change_amount)
            };
            if target_period > 0x7ff {
                self.muting = true;
            }
            if !self.muting {
                timer.period = target_period;
            }
        }
        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }

    pub fn amplitude(&self) -> f32 {
        if self.muting {
            0.0
        } else {
            1.0
        }
    }

    pub fn set_sweep(&mut self, data: u8) {
        self.enabled = data & 0x80 != 0;
        self.period = (data >> 4) & 0x7;
        self.negate = data & 0x08 != 0;
        self.shift = data & 0x07;
        self.reload = true;
    }
}
