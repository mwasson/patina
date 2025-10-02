mod ppu;
mod tile;
mod palette;
mod ppu_registers;
mod ppu_internal_registers;

pub mod ppu_listener;

pub use ppu::{PPU, NametableMirroring};
pub use ppu_registers::PPURegister;
pub use ppu_internal_registers::PPUInternalRegisters;
pub use tile::{_index_to_pixel, Tile};

pub const OAM_SIZE : usize = 256;
pub const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
/* number of lines at the top of the screen to not actually show */
pub const OVERSCAN : u8 = 10;
pub const DISPLAY_WIDTH : u32 = 256;
pub const DISPLAY_HEIGHT : u32 = 240-OVERSCAN as u32;
pub const WRITE_BUFFER_SIZE : usize = (DISPLAY_WIDTH as usize)*(DISPLAY_HEIGHT as usize)*4;

type OAM = [u8; OAM_SIZE];
pub type VRAM = [u8; PPU_MEMORY_SIZE];

pub type WriteBuffer = [u8; WRITE_BUFFER_SIZE];