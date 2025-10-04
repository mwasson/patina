use crate::mapper::Mapper;

pub struct Tile {
    tile_addr: u16,
}

impl Tile {
    pub fn new(tile_addr: u16) -> Tile {
        Tile { tile_addr }
    }

    /* TODO serious comments required */
    #[allow(dead_code)]
    pub fn render(&self, mapper: &Box<dyn Mapper>) -> [u8; 4 * 8 * 8] {
        let mut out = [0; 4 * 8 * 8];

        for row in 0..8 {
            for col in 0..8 {
                let val = self.pixel_intensity(mapper, col, row);
                let result: u8 = (val as u16 * 256 / 4) as u8;
                let out_index = pixel_to_index(8, col as usize, row as usize);
                out[out_index..(out_index + 4)].copy_from_slice(&[result, result, result, 0xff]);
            }
        }

        out
    }

    #[allow(dead_code)]
    pub fn stamp(
        &self,
        mapper: &Box<dyn Mapper>,
        write_buffer: &mut [u8],
        width: usize,
        x: usize,
        y: usize,
    ) {
        let mut tile_index = 0;
        for chunk in self.render(mapper).chunks_exact(4) {
            let tile_xy = index_to_pixel(8, tile_index);
            let index = pixel_to_index(width, x + tile_xy.0, y + tile_xy.1);
            write_buffer[index..index + 4].copy_from_slice(chunk);
            tile_index += 1;
        }
    }

    #[inline(never)]
    pub fn pixel_intensity(&self, mapper: &Box<dyn Mapper>, x: u8, y: u8) -> u8 {
        let rev_x = 7 - x;
        /* double tall sprites are actually two regular 8x8 tiles glued together,
         * so for the second half we need to increment values by 8 to index it correctly
         */
        let y_row = if y > 7 { y + 8 } else { y } as u16;
        let big = mapper.read_chr(self.tile_addr + y_row + 8) >> rev_x;
        let small = mapper.read_chr(self.tile_addr + y_row) >> rev_x;
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
#[allow(dead_code)]
pub fn index_to_pixel(width: usize, index: usize) -> (usize, usize) {
    (index % width, index / width)
}

/* TODO comment */
#[allow(dead_code)]
pub fn pixel_to_index(width: usize, x: usize, y: usize) -> usize {
    4 * (width * y + x)
}
