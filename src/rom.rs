pub struct Rom {
    pub prg_data: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub byte_6_flags: u8, /* TODO: split these out */
    pub byte_7_flags: u8, /* TODO: split these out */
    pub trainer: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub tv_system: u8, /* TODO: make into a boolean or enum */
}  
