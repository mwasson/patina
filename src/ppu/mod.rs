mod ppu_state;
mod tile;
mod palette;
mod ppu_registers;
mod ppu_scroll_state;

pub mod ppu_to_cpu_message;

pub use crate::cpu::ppu_listener::PPUListener;
pub use ppu_state::PPUState;
pub use ppu_registers::PPURegister;
pub use ppu_scroll_state::PPUScrollState;
pub use tile::{index_to_pixel, pixel_to_index, Tile};

pub const OAM_SIZE : usize = 256;
pub const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
pub const WRITE_BUFFER_SIZE : usize = 256*240*4;

type OAM = [u8; OAM_SIZE];
pub type VRAM = [u8; PPU_MEMORY_SIZE];

pub type WriteBuffer = [u8; WRITE_BUFFER_SIZE];