pub(crate) enum PpuToCpuMessage
{
    NMI,
    PpuStatus(u8),
}