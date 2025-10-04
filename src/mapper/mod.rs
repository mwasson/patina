mod mapper;
mod mmc1;
mod nrom;
mod uxrom;

use nrom::NROM;
use std::cell::RefCell;
use std::rc::Rc;

use crate::mapper::mmc1::MMC1;
use crate::mapper::uxrom::UxROM;
use crate::rom::Rom;
pub use mapper::Mapper;

pub fn load_mapper(mapper_num: u8, rom: &Rom) -> Rc<RefCell<Box<dyn Mapper>>> {
    let mapper: Box<dyn Mapper> = match mapper_num {
        0 => Box::new(NROM::new(rom)),
        1 => Box::new(MMC1::new(rom)),
        2 => Box::new(UxROM::new(rom)),
        _ => todo!("mapper {mapper_num}"),
    };
    Rc::new(RefCell::new(mapper))
}
