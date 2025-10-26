use crate::mapper::Mapper;
use crate::ppu::NametableMirroring;

pub struct TestMapper {
    memory: Box<[u8; 0x8000]>,
}

impl TestMapper {
    pub fn new() -> Self {
        TestMapper {
            memory: Box::new([0; 0x8000]),
        }
    }

    fn map_address(address: u16) -> usize {
        (address - 0x8000) as usize
    }
}

impl Mapper for TestMapper {
    fn read_prg(&self, address: u16) -> u8 {
        if address < 0x8000 {
            return 0;
        }
        self.memory[Self::map_address(address)]
    }

    fn read_prg_slice(&self, address: u16, _size: usize) -> &[u8] {
        let map_address = Self::map_address(address);

        &self.memory[map_address..map_address + _size]
    }

    fn write_prg(&mut self, address: u16, value: u8) {
        if address >= 0x8000 {
            self.memory[Self::map_address(address)] = value;
        } else {
            panic!("writing to address test mapper doesn't cover");
        }
    }

    fn read_chr(&self, _address: u16) -> u8 {
        panic!("should never be called")
    }

    fn write_chr(&mut self, _address: u16, _value: u8) {
        panic!("should never be called")
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        panic!("should never be called")
    }
}
