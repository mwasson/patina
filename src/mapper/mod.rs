mod bank_array;
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

/* common bank sizes; u16 since they must fit in the CPU address space */
const SIZE_4_KB: usize = 12;
const SIZE_8_KB: usize = 13;
const SIZE_16_KB: usize = 14;
const SIZE_32_KB: usize = 15;

pub fn load_mapper(mapper_num: u8, rom: &Rom) -> Box<dyn Mapper> {
    match mapper_num {
        0 => Box::new(NROM::new(rom)),
        1 => Box::new(MMC1::new(rom)),
        2 => Box::new(UxROM::new(rom)),
        _ => todo!("mapper {mapper_num}"),
    }
}
