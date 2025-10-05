pub struct Envelope {
    decay_level: u8,
    period_or_volume: u8,
    divider: u8,
    constant_volume: bool,
    start_flag: bool,
    loop_flag: bool,
}

impl Envelope {
    pub fn new() -> Envelope {
        Envelope {
            decay_level: 0,
            period_or_volume: 0,
            divider: 0,
            constant_volume: false,
            start_flag: false,
            loop_flag: false,
        }
    }

    #[cfg_attr(debug_assertions, inline(never))]
    pub fn clock(&mut self) {
        /* if the start flag is set, reset the envelope: decay level is maxed out, and
         * the divider goes back to the beignning of the period
         */
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider = self.period_or_volume;
        /* whenever the divider reaches zero, we loop around and then modify the decay level
         * appropriately: we either decrement it, or if the loop flag is set,
         * we max it out again (creating a sawtooth wave)
         */
        } else if self.divider == 0 {
            self.divider = self.period_or_volume;
            if self.decay_level == 0 && self.loop_flag
            /* halt flag is also loop flag */
            {
                self.decay_level = 15;
            } else if self.decay_level > 0 {
                self.decay_level -= 1;
            }
        /* otherwise decrement the divider */
        } else {
            self.divider -= 1;
        }
    }

    pub fn amplitude(&self) -> f32 {
        (if self.constant_volume {
            self.period_or_volume
        } else {
            self.decay_level
        }) as f32
    }

    pub fn set_envelope(&mut self, data: u8) {
        self.period_or_volume = data & 0xf;
        self.constant_volume = data & 0x10 != 0;
        self.loop_flag = data & 0x20 != 0;
    }

    pub fn start(&mut self) {
        self.start_flag = true;
        self.decay_level = 0;
    }
}
