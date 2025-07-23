use crate::ppu::{Tile, index_to_pixel, pixel_to_index};

pub struct Rom {
    pub prg_data: Vec<u8>,
    pub chr_data: Vec<u8>,
    pub byte_6_flags: u8, /* TODO: split these out */
    pub byte_7_flags: u8, /* TODO: split these out */
    pub trainer: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub tv_system: u8, /* TODO: make into a boolean or enum */
}

impl Rom {
    /* TODO this stuff should *all* be somewhere else */
    pub fn render(&self, write_buffer: &mut [u8], width: usize) {
        Rom::render_pattern_table(&self.chr_data[0..(256*16)], write_buffer, width, 0);
        Rom::render_pattern_table(&self.chr_data[(256*16)..(256*32)], write_buffer, width, 128);
    }

    pub fn render_pattern_table(pattern_table: &[u8], write_buffer: &mut [u8], width: usize, start_x: usize) {
        for i in (0..256) {
            let xy = index_to_pixel(16, i);
            let data_start = i*16;
            let mut tile_data = [0 as u8; 16];
            tile_data.copy_from_slice(&pattern_table[data_start..data_start + 16]);
            let tile = Tile::from_memory(tile_data);
            tile.stamp(write_buffer, width, xy.0*8+start_x, xy.1*8);
        }
    }
}
