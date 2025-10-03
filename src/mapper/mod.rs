mod mapper;
mod nrom;
mod mmc1;

use std::cell::RefCell;
use std::rc::Rc;
use nrom::NROM;

pub use mapper::Mapper;
use crate::mapper::mmc1::MMC1;
use crate::rom::Rom;

pub fn load_mapper(mapper_num: u8, rom: &Rom) -> Rc<RefCell<Box<dyn Mapper>>> {
    let mapper : Box<dyn Mapper> = match mapper_num {
        0 => Box::new(NROM::new(rom)),
        1 => Box::new(MMC1::new(rom)),
        _ => todo!("mapper {mapper_num}"),
    };
    Rc::new(RefCell::new(mapper))
}