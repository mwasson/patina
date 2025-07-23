mod ppu_state;
mod tile;
mod palette;
mod ppu_registers;
mod ppu_listener;

pub use ppu_listener::PPUListener;
pub use ppu_state::PPUState;
pub use ppu_registers::PPURegister;
pub use tile::{Tile, index_to_pixel, pixel_to_index};

pub const OAM_SIZE : usize = 256;
pub const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
pub const WRITE_BUFFER_SIZE : usize = 256*240*4;

type OAM = [u8; OAM_SIZE];
type VRAM = [u8; PPU_MEMORY_SIZE];

pub type WriteBuffer = [u8; WRITE_BUFFER_SIZE];