pub trait Mapper {
    fn read(&self, address: u16) -> u8;

    fn read_slice(&self, address: u16, size: usize) -> &[u8];

    fn write(&mut self, address: u16, value: u8);
}