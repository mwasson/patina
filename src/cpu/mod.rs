mod addressing_mode;
mod controller;
mod core_memory;
mod cpu;
mod instruction;
mod operation;
mod status_flag;

pub use crate::cpu::instruction::from_opcode;
pub use addressing_mode::AddressingMode;
pub use controller::Controller;
pub use core_memory::CoreMemory;
pub use core_memory::MemoryListener;
pub use cpu::CPU;
pub use instruction::RealizedInstruction;
pub use status_flag::StatusFlag;
pub const MEMORY_SIZE: usize = 1 << 11; /* 2kB onboard RAM */

const NMI_HANDLER_LOCATION: u16 = 0xfffa;
const INITIAL_PC_LOCATION: u16 = 0xfffc;
const IRQ_HANDLER_LOCATION: u16 = 0xfffe;

/**
 * Converts a pair of bytes into a u16 to look up an address in memory.
 * The 6502 is little-endian, so this expects the low-order byte first.
 * addr(0xCD, 0xAB) returns 0xABCD.
 */
fn addr(lo_byte: u8, hi_byte: u8) -> u16 {
    ((hi_byte as u16) << 8) + (lo_byte as u16)
}

/**
 * Zero-page address operations take a single-byte and result in an
 * address on the first page of memory, which has addresses that begin
 * with 0x00. If this is passed in 0xAB, it returns 0x00AB. In effect this
 * is just a cast, but wrapping it as a function makes the goal clearer.
 */
fn zero_page_addr(b1: u8) -> u16 {
    b1 as u16
}
