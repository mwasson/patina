mod mapper;
mod nrom;

use nrom::NROM;

pub use mapper::Mapper;
use crate::rom::Rom;

pub fn load_mapper(mapper_num: u8, rom: &Rom) -> Box<dyn Mapper> {
    Box::new(match mapper_num {
        0 => NROM::new(rom), /* nrom */
        _ => todo!("mapper {mapper_num}"),
    })
}