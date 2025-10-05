use crate::ppu::{NametableMirroring, Tile};

pub trait Mapper {
    fn read_prg(&self, address: u16) -> u8;

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8];

    fn write_prg(&mut self, address: u16, value: u8);

    fn read_chr(&self, address: u16) -> u8;

    #[cfg_attr(debug_assertions, inline(never))]
    fn read_tile(&self, tile_index: u8, pattern_table_num: u8) -> Tile {
        let pattern_table_base = 0x1000u16 * pattern_table_num as u16;
        let tile_start = pattern_table_base + (tile_index as u16 * 16);
        Tile::new(tile_start)
    }

    fn write_chr(&mut self, address: u16, value: u8);

    fn get_nametable_mirroring(&self) -> NametableMirroring;
}
