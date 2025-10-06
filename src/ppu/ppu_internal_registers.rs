#[derive(Debug, Default, Clone)]
pub struct PPUInternalRegisters {
    pub v: u16,
    pub t: u16,
    pub x: u8,
    pub w: bool,
}

impl PPUInternalRegisters {
    pub fn get_coarse_x(&self) -> u8 {
        (self.v & 0x1f) as u8
    }

    pub fn get_coarse_x_tmp(&self) -> u8 {
        (self.t & 0x1f) as u8
    }

    pub fn set_coarse_x(&mut self, data: u8) {
        PPUInternalRegisters::set_coarse_x_internal(&mut self.v, data);
    }

    pub fn set_coarse_x_t(&mut self, data: u8) {
        PPUInternalRegisters::set_coarse_x_internal(&mut self.t, data);
    }

    fn set_coarse_x_internal(ptr: &mut u16, data: u8) {
        *ptr = (*ptr & !0x1f) | (data as u16 & 0x1f);
    }

    pub fn get_fine_x(&self) -> u8 {
        self.x
    }

    pub fn set_fine_x(&mut self, data: u8) {
        self.x = data
    }

    pub fn get_coarse_y(&self) -> u8 {
        ((self.v >> 5) & 0x1f) as u8
    }

    pub fn get_coarse_y_tmp(&self) -> u8 {
        ((self.t >> 5) & 0x1f) as u8
    }

    pub fn set_coarse_y(&mut self, data: u8) {
        PPUInternalRegisters::set_coarse_y_internal(&mut self.v, data);
    }

    pub fn set_coarse_y_t(&mut self, data: u8) {
        PPUInternalRegisters::set_coarse_y_internal(&mut self.t, data);
    }

    pub fn set_coarse_y_internal(ptr: &mut u16, data: u8) {
        *ptr = (*ptr & !0x03e0) | ((data as u16 & 0x1f) << 5);
    }

    pub fn get_fine_y(&self) -> u8 {
        ((self.v >> 12) & 0x7) as u8
    }

    pub fn get_fine_y_tmp(&self) -> u8 {
        ((self.t >> 12) & 0x7) as u8
    }

    pub fn get_nametable(&self) -> u8 {
        ((self.v >> 10) & 0x3) as u8
    }

    pub fn get_nametable_t(&self) -> u8 {
        ((self.t >> 10) & 0x3) as u8
    }

    pub fn set_nametable_t(&mut self, data: u8) {
        PPUInternalRegisters::set_nametable_internal(&mut self.t, data);
    }

    pub fn set_nametable(&mut self, data: u8) {
        PPUInternalRegisters::set_nametable_internal(&mut self.v, data);
    }

    fn set_nametable_internal(ptr: &mut u16, data: u8) {
        *ptr = (*ptr & !0x0c00) | ((data as u16 & 0x3) << 10);
    }

    pub fn set_fine_y(&mut self, data: u8) {
        PPUInternalRegisters::set_fine_y_internal(&mut self.v, data);
    }

    pub fn set_fine_y_t(&mut self, data: u8) {
        PPUInternalRegisters::set_fine_y_internal(&mut self.t, data);
    }

    fn set_fine_y_internal(ptr: &mut u16, data: u8) {
        *ptr = (*ptr & 0x0fff) | ((data as u16 & 0x7) << 12)
    }

    pub fn is_first_write(&self) -> bool {
        !self.w
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn y_increment(&mut self) {
        let fine_y = self.get_fine_y();
        let coarse_y = self.get_coarse_y();

        if fine_y < 7 {
            /* keep on incrementing fine_y until we can't */
            self.set_fine_y(fine_y + 1);
        } else {
            /* wrap fine y, go to next vertical name table, being careful of attr table */
            self.set_fine_y(0);
            if coarse_y == 29 {
                self.set_coarse_y(0);
                self.set_nametable(self.get_nametable() ^ 0x2); /* switch vertical nametable */
            } else if coarse_y == 31 {
                self.set_coarse_y(0);
            } else {
                self.set_coarse_y(coarse_y + 1);
            }
        }
    }

    pub fn coarse_x_increment(&mut self) {
        let coarse_x = self.get_coarse_x();
        if coarse_x == 31 {
            self.set_coarse_x(0);
            self.set_nametable(self.get_nametable() ^ 0x1); /* switch horizontal nametable */
        } else {
            self.set_coarse_x(coarse_x + 1);
        }
    }

    pub fn copy_x_bits(&mut self) {
        self.set_coarse_x(self.get_coarse_x_tmp());
        self.set_nametable((self.get_nametable() & 0x2) | (self.get_nametable_t() & 0x1));
    }

    pub fn copy_y_bits(&mut self) {
        self.set_coarse_y(self.get_coarse_y_tmp());
        self.set_fine_y(self.get_fine_y_tmp());
        self.set_nametable((self.get_nametable() & 0x1) | (self.get_nametable_t() & 0x2));
    }
}
