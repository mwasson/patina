#[derive(Debug)]
pub enum PPURegister {
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

impl PPURegister {
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

    pub fn from_addr(addr: u16) -> Option<PPURegister> {
        match addr {
            0x2000 => Some(PPURegister::PPUCTRL),
            0x2001 => Some(PPURegister::PPUMASK),
            0x2002 => Some(PPURegister::PPUSTATUS),
            0x2003 => Some(PPURegister::OAMADDR),
            0x2004 => Some(PPURegister::OAMDATA),
            0x2005 => Some(PPURegister::PPUSCROLL),
            0x2006 => Some(PPURegister::PPUADDR),
            0x2007 => Some(PPURegister::PPUDATA),
            0x4014 => Some(PPURegister::OAMDMA),
            _ => None,
        }
    }
}
