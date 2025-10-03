use crate::ppu::Tile;

pub trait Mapper {
    fn read_prg(&self, address: u16) -> u8;

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8];

    fn write_prg(&mut self, address: u16, value: u8);
    
    fn read_chr(&self, address: u16) -> u8;
    
    fn read_tile(&self, tile_index: u8, pattern_table_num: usize) -> Tile;
    
    fn write_chr(&mut self, address: u16, value: u8);
}