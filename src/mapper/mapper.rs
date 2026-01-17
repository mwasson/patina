use crate::ppu::NametableMirroring;

pub trait Mapper: Send {
    fn read_prg(&self, address: u16) -> u8;

    fn read_prg_slice(&self, address: u16, size: usize) -> &[u8];

    fn write_prg(&mut self, address: u16, value: u8);

    fn read_chr(&self, address: u16) -> u8;

    fn write_chr(&mut self, address: u16, value: u8);

    fn get_nametable_mirroring(&self) -> NametableMirroring;

    /**
     * Returns the current value of the data that would be saved in RAM, if it exists,
     * or None if this mapper doesn't support it.
     */
    fn get_save_data(&self) -> Option<Vec<u8>> {
        None
    }

    /**
     * Sets the current save RAM data. If the mapper does not support save RAM,
     * this has no effect.
     */
    fn set_save_data(&mut self, _data: &Vec<u8>) {}
}
