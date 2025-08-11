use crate::ppu::OAM_SIZE;

pub(crate) enum CpuToPpuMessage
{
    Memory(usize, u8), /* addr, data */
    Oam([u8; OAM_SIZE]),
    PpuCtrl(u8), /* writes to PPUCTRL */
    PpuMask(u8), /* writes to PPUMASK */
}