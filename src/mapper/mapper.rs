use crate::ppu::NametableMirroring;

pub trait Mapper: Send {
    fn read_prg(&self, address: u16) -> u8;

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8];

    fn write_prg(&mut self, address: u16, value: u8);

    fn read_chr(&self, address: u16) -> u8;

    fn write_chr(&mut self, address: u16, value: u8);

    fn get_nametable_mirroring(&self) -> NametableMirroring;
}
