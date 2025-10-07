use crate::mapper::Mapper;
use bit_reverse::LookupReverse;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Tile {
    tile_addr: u16,
    mapper: Rc<RefCell<Box<dyn Mapper>>>,
    cached_y: u8, /* used to determine if cached_big and cached_small are valid */
    cached_big: u8,
    cached_small: u8,
}

impl Tile {
    pub fn new(tile_addr: u16, mapper: Rc<RefCell<Box<dyn Mapper>>>) -> Tile {
        Tile {
            tile_addr,
            cached_y: 0xff, /* initial invalid value */
            cached_big: 0,
            cached_small: 0,
            mapper,
        }
    }

    /* TODO serious comments required */
    #[allow(dead_code)]
    pub fn render(&mut self) -> [u8; 4 * 8 * 8] {
        let mut out = [0; 4 * 8 * 8];

        for row in 0..8 {
            for col in 0..8 {
                let val = self.pixel_intensity(col, row);
                let result: u8 = (val as u16 * 256 / 4) as u8;
                let out_index = pixel_to_index(8, col as usize, row as usize);
                out[out_index..(out_index + 4)].copy_from_slice(&[result, result, result, 0xff]);
            }
        }

        out
    }

    #[allow(dead_code)]
    pub fn stamp(&mut self, write_buffer: &mut [u8], width: usize, x: usize, y: usize) {
        let mut tile_index = 0;
        for chunk in self.render().chunks_exact(4) {
            let tile_xy = index_to_pixel(8, tile_index);
            let index = pixel_to_index(width, x + tile_xy.0, y + tile_xy.1);
            write_buffer[index..index + 4].copy_from_slice(chunk);
            tile_index += 1;
        }
    }

    pub fn pixel_intensity(&mut self, x: u8, y: u8) -> u8 {
        self.populate_cache(y);
        let big = self.cached_big >> x;
        let small = self.cached_small >> x;
        ((big & 1) << 1) | (small & 1)
    }

    fn populate_cache(&mut self, y: u8) {
        if self.cached_y != y {
            self.cached_y = y;
            /* double tall sprites are actually two regular 8x8 tiles glued together,
             * so for the second half we need to increment values by 8 to index it correctly
             */
            let y_row = self.tile_addr + (if y > 8 { y + 8 } else { y } as u16);
            /* memory stores bits in the opposite order of x indexing; reversing them here
             * to avoid a subtraction later */
            let mapper = self.mapper.borrow();
            self.cached_big = LookupReverse::swap_bits(mapper.read_chr(y_row + 8));
            self.cached_small = LookupReverse::swap_bits(mapper.read_chr(y_row));
        }
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
