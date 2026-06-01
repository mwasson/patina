use crate::mapper::Mapper;
use crate::ppu::{NametableMirroring, PPU, WRITE_BUFFER_SIZE};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct MockMapper {
    pub chr: [u8; 0x2000],
    mirroring: NametableMirroring,
}

impl MockMapper {
    pub fn new(mirroring: NametableMirroring) -> Self {
        MockMapper { chr: [0; 0x2000], mirroring }
    }
}

impl Mapper for MockMapper {
    fn read_prg(&self, _address: u16) -> u8 {
        0
    }

    fn read_prg_slice(&self, _address: u16, _size: usize) -> &[u8] {
        panic!("MockMapper: PRG slice not supported")
    }

    fn write_prg(&mut self, _address: u16, _value: u8) {}

    fn read_chr(&self, address: u16) -> u8 {
        self.chr[address as usize]
    }

    fn write_chr(&mut self, address: u16, value: u8) {
        self.chr[address as usize] = value;
    }

    fn get_nametable_mirroring(&self) -> NametableMirroring {
        self.mirroring.clone()
    }
}

pub fn make_ppu_with_buffer(
    mirroring: NametableMirroring,
) -> (Rc<RefCell<PPU>>, Arc<Mutex<[u8; WRITE_BUFFER_SIZE]>>) {
    let write_buffer = Arc::new(Mutex::new([0u8; WRITE_BUFFER_SIZE]));
    let mapper: Rc<RefCell<Box<dyn Mapper>>> =
        Rc::new(RefCell::new(Box::new(MockMapper::new(mirroring))));
    let ppu = PPU::new(write_buffer.clone(), mapper);
    (ppu, write_buffer)
}

pub fn make_ppu(mirroring: NametableMirroring) -> Rc<RefCell<PPU>> {
    make_ppu_with_buffer(mirroring).0
}
