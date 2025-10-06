use crate::mapper::Mapper;
use crate::ppu::PPU;

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    tile_addr: u16,
    cached_row: u8,
    cached_small: u8,
    cached_big: u8,
}

impl Tile {
    pub fn new(tile_addr: u16) -> Tile {
        Tile {
            tile_addr,
            cached_row: 0xff, /* invalid value */
            cached_small: 0,
            cached_big: 0,
        }
    }

    /* TODO serious comments required */
    #[allow(dead_code)]
    pub fn render(&mut self, ppu: &PPU) -> [u8; 4 * 8 * 8] {
        let mut out = [0; 4 * 8 * 8];

        for row in 0..8 {
            for col in 0..8 {
                let val = self.pixel_intensity(ppu, col, row);
                let result: u8 = (val as u16 * 256 / 4) as u8;
                let out_index = pixel_to_index(8, col as usize, row as usize);
                out[out_index..(out_index + 4)].copy_from_slice(&[result, result, result, 0xff]);
            }
        }

        out
    }

    #[allow(dead_code)]
    pub fn stamp(&mut self, ppu: &PPU, write_buffer: &mut [u8], width: usize, x: usize, y: usize) {
        let mut tile_index = 0;
        for chunk in self.render(ppu).chunks_exact(4) {
            let tile_xy = index_to_pixel(8, tile_index);
            let index = pixel_to_index(width, x + tile_xy.0, y + tile_xy.1);
            write_buffer[index..index + 4].copy_from_slice(chunk);
            tile_index += 1;
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn pixel_intensity(&mut self, ppu: &PPU, x: u8, y: u8) -> u8 {
        let rev_x = 7 - x;
        /* double tall sprites are actually two regular 8x8 tiles glued together,
         * so for the second half we need to increment values by 8 to index it correctly
         */
        if self.cached_row != y {
            self.cached_row = y;
            let y_row = if y > 7 { y + 8 } else { y } as u16;
            let mapper = ppu.mapper.borrow();
            self.cached_big = mapper.read_chr(self.tile_addr + y_row + 8);
            self.cached_small = mapper.read_chr(self.tile_addr + y_row);
        }
        ((self.cached_big >> rev_x & 1) << 1) | (self.cached_small >> rev_x & 1)
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
