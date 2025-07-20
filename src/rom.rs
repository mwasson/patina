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
        for i in (0..256) {
            let xy = Rom::index_to_pixel(16, i);
            let data_start = i*16;
            Rom::stamp_tile(write_buffer, width, xy.0*8, xy.1*8, &Rom::render_tile(&self.chr_data[data_start..data_start+16]));
        }
        for i in (256..512) {
            let xy = Rom::index_to_pixel(16, i-256);
            let data_start = i*16;
            Rom::stamp_tile(write_buffer, width, xy.0*8+128, xy.1*8, &Rom::render_tile(&self.chr_data[data_start..data_start+16]));
        }
    }

    fn stamp_tile(write_buffer: &mut [u8], width: usize, x: usize, y: usize, tile:&[u8]) {
        let mut tile_index = 0;
        for chunk in tile.chunks_exact(4) {
            let tile_xy = Rom::index_to_pixel(8, tile_index);
            let index = Rom::pixel_to_index(width, x+tile_xy.0, y+tile_xy.1);
            write_buffer[index..index+4].copy_from_slice(chunk);
            tile_index += 1;
        }
    }

    fn index_to_pixel(width:usize, index:usize) -> (usize, usize) {
        (index % width, index/width)
    }

    fn pixel_to_index(width:usize, x:usize, y:usize) -> usize {
        4*(width*y + x)
    }

    /* TODO serious comments required */
    fn render_tile(tile: &[u8]) -> [u8; 4*8*8]{
        let mut out = [0; 4*8*8];

        for row in 0..8 {
            for col in 0..8 {
                let val = (tile[row] & (1 << 7-col)).count_ones() + 2*(tile[8+row] & 1 << 7-col).count_ones();
                let result:u8 = (val * 256 / 4) as u8;
                let out_index = Rom::pixel_to_index(8,col,row);
                // out[out_index..(out_index+4)].copy_from_slice(&[255,255,255,0xff]);
                out[out_index..(out_index+4)].copy_from_slice(&[result,result,result,0xff]);
            }
        }

        out
    }
}
