use crate::ppu::{NametableMirroring, Tile};

pub trait Mapper {
    fn read_prg(&self, address: u16) -> u8;

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8];

    fn write_prg(&mut self, address: u16, value: u8);

    fn read_chr(&self, address: u16) -> u8;

    #[inline(never)]
    fn read_tile(&self, tile_index: u8, pattern_table_num: usize, double_height: bool) -> Tile {
        let pattern_table_base : usize = 0x1000 * pattern_table_num;
        let len = if double_height { 32 } else { 16 };
        let tile_start = pattern_table_base + (tile_index as usize * 16);
        let mut memcopy = Vec::with_capacity(len);
        for i in 0..len {
            memcopy.push(self.read_chr(tile_start as u16 + i as u16));
        }
        Tile::from_memory(memcopy)
    }

    fn write_chr(&mut self, address: u16, value: u8);

    fn get_nametable_mirroring(&self) -> NametableMirroring;
}