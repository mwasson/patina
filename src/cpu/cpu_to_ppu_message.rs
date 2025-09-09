use crate::ppu::{PPUScrollState, OAM_SIZE};

pub(crate) enum CpuToPpuMessage
{
    Memory(usize, u8), /* addr, data */
    Oam([u8; OAM_SIZE]),
    PpuCtrl(u8), /* writes to PPUCTRL */
    PpuMask(u8), /* writes to PPUMASK */
    ScrollX(u8, u8), /* coarse x, fine x */
    ScrollY(u8, u8), /* coarse y, fine y */
}