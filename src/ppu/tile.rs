#[derive(Debug)]
pub struct Tile
{
    data: Vec<u8>
}

impl Tile
{
    pub fn from_memory(memory: Vec<u8>) -> Tile {
        Tile {
            data: memory
        }
    }

    /* TODO serious comments required */
    pub fn _render(&self) -> [u8; 4*8*8]{
        let mut out = [0; 4*8*8];

        for row in 0..8 {
            for col in 0..8 {
                let val = self.pixel_intensity(col, row);
                let result:u8 = (val as u16 * 256 / 4) as u8;
                let out_index = _pixel_to_index(8,col,row);
                out[out_index..(out_index+4)].copy_from_slice(&[result,result,result,0xff]);
            }
        }

        out
    }

    pub fn _stamp(&self, write_buffer: &mut [u8], width: usize, x: usize, y: usize) {
        let mut tile_index = 0;
        for chunk in self._render().chunks_exact(4) {
            let tile_xy = _index_to_pixel(8, tile_index);
            let index = _pixel_to_index(width, x+tile_xy.0, y+tile_xy.1);
            write_buffer[index..index+4].copy_from_slice(chunk);
            tile_index += 1;
        }
    }

    #[inline(never)]
    pub fn pixel_intensity(&self, x:usize, y:usize) -> u8 {
        let rev_x = 7-x;
        /* double tall sprites are actually two regular 8x8 tiles glued together,
         * so for the second half we need to increment values by 8 to index it correctly
         */
        let y_row = if y > 7 { y + 8 } else { y };
        let big = self.data[y_row+8] >> rev_x;
        let small = self.data[y_row] >> rev_x;
        ((big & 1) << 1) | (small & 1)
        // self.bit_set(self.data[y], x) + 2*self.bit_set(self.data[8+y], x)
    }

    /* checks if a given bit in a bit array is set, and returns 1 if true, 0 otherwise;
     * in this case the highest order bit is the 0th column, lowest order is the 7th column
     * as we work from left to right */
    fn _bit_set(&self, bit_array: u8, col: usize) -> u8 {
        (bit_array & (1 << 7 - col)) >> 7 - col
    }
}


/* TODO comment */
pub fn _index_to_pixel(width:usize, index:usize) -> (usize, usize) {
    (index % width, index/width)
}

/* TODO comment */
pub fn _pixel_to_index(width:usize, x:usize, y:usize) -> usize {
    4*(width*y + x)
}