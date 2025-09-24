use crate::apu::timer::Timer;

pub struct Sweep {
    divider: u8,
    enabled: bool,
    negate: bool,
    period: u8,
    reload: bool,
    shift: u8,
}

impl Sweep {
    pub fn new() -> Sweep {
        Sweep {
            divider: 0,
            enabled: false,
            negate: false,
            period: 0,
            reload: false,
            shift: 0,
        }
    }

    pub fn clock(&mut self, timer: &mut Timer) {
        if self.enabled && self.shift != 0 && self.divider == 0 {
            let change_amount = timer.period >> self.shift;
            /* TODO: handle pulse 1 vs pulse 2 differences */
            let target_period = if self.negate {
                timer.period.saturating_sub(change_amount)
            } else {
                timer.period.wrapping_add(change_amount)
            };
            timer.period = target_period;
        }
        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }

    pub fn set_sweep(&mut self, data: u8) {
        self.enabled = data & 0x80 == 0;
        self.period = (data >> 4) & 0x7;
        self.negate = data & 0x08 != 0;
        self.shift = data & 0x07;
        self.reload = true;
        self.divider = self.period;
    }
}