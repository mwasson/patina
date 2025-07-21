use std::ops::Deref;
use crate::cpu::{CoreMemory, ProgramState};

pub enum PPURegister
{
    PPUCTRL,
    PPUMASK,
    PPUSTATUS,
    OAMADDR,
    OAMDATA,
    PPUSCROLL,
    PPUADDR,
    PPUDATA,
    OAMDMA,
}

impl PPURegister
{
    pub fn address(register: &PPURegister) -> u16 {
        match register {
            PPURegister::PPUCTRL => 0x2000,
            PPURegister::PPUMASK => 0x2001,
            PPURegister::PPUSTATUS => 0x2002,
            PPURegister::OAMADDR => 0x2003,
            PPURegister::OAMDATA => 0x2004,
            PPURegister::PPUSCROLL => 0x2005,
            PPURegister::PPUADDR => 0x2006,
            PPURegister::PPUDATA => 0x2007,
            PPURegister::OAMDMA => 0x4014,
        }
    }
    pub fn read(&self, memory: &CoreMemory) -> u8 {
        memory.read(PPURegister::address(self))
    }

    pub fn write(&self, memory: &mut CoreMemory, data: u8) {
        memory.write(PPURegister::address(self), data);
    }
}