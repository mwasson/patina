mod ppu;
mod tile;
mod palette;
mod ppu_registers;
mod ppu_internal_registers;

pub mod ppu_listener;

pub use ppu::{PPU, NametableMirroring};
pub use ppu_registers::PPURegister;
pub use ppu_internal_registers::PPUInternalRegisters;
pub use tile::Tile;

const OAM_SIZE : usize = 256;
/* 4kb VRAM covering the entirety of the nametable space; in reality only 2kb is used but this
 * makes addressing easier
 */
const VRAM_SIZE : usize = 1 << 11;
const PALETTE_MEMORY_SIZE: usize = 32;
/* number of lines at the top of the screen to not actually show */
const OVERSCAN : u8 = 10;

pub const DISPLAY_WIDTH : u32 = 256;
pub const DISPLAY_HEIGHT : u32 = 240-OVERSCAN as u32;
pub const WRITE_BUFFER_SIZE : usize = (DISPLAY_WIDTH as usize)*(DISPLAY_HEIGHT as usize)*4;

type OAM = [u8; OAM_SIZE];

pub type WriteBuffer = [u8; WRITE_BUFFER_SIZE];