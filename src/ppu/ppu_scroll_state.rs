#[derive(Default, Clone)]
pub struct PPUScrollState
{
    pub coarse_x: u8,
    pub coarse_y: u8,
    pub fine_x: u8,
    pub fine_y: u8,
    pub nametable: u8,
}

impl PPUScrollState
{
    pub fn y_increment(&mut self) {
        if self.fine_y < 7 {
            /* keep on incrementing fine_y until we can't */
            self.fine_y += 1;
        } else {
            /* wrap fine y, go to next vertical name table, being careful of attr table */
            self.fine_y = 0;
            if (self.coarse_y == 29) {
                self.coarse_y = 0;
                self.nametable = (self.nametable ^ 0x2) | (self.nametable & 0x1);/* switch vertical nametable */
            } else if (self.coarse_y == 31) {
                self.coarse_y = 0;
            } else {
                self.coarse_y += 1;
            }
        }
    }

    pub fn coarse_x_increment(&mut self) {
        if self.coarse_x == 31 {
            self.coarse_x = 0;
            self.nametable = (self.nametable ^ 0x1) | (self.nametable & 0x2);/* switch horizontal nametable */
        } else {
            self.coarse_x += 1;
        }
    }
}