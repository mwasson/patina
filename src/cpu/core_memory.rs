use std::cell::RefCell;
use std::rc::Rc;
use fnv::FnvHashMap;
use crate::cpu::MEMORY_SIZE;
use crate::rom::Rom;

pub trait MemoryListener {
    fn get_addresses(&self) -> Vec<u16>;

    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8;
    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8);
}

pub struct CoreMemory {
    memory: [u8; MEMORY_SIZE],
    nmi_flag: bool, /* a convenience, to avoid a PPU dependency on the CPU */
    listeners: FnvHashMap<u16, Rc<RefCell<dyn MemoryListener>>>
}

impl CoreMemory {
    pub fn new(rom: &Rom) -> CoreMemory {
        /* TODO: handling RAM, mappers, etc. */
        let mut memory = [0; MEMORY_SIZE];
        memory[(0x10000 - rom.prg_data.len())..0x10000].copy_from_slice(&*rom.prg_data);

        CoreMemory {
            memory,
            nmi_flag: false,
            listeners:  FnvHashMap::with_capacity_and_hasher(10, Default::default())
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        let mapped_addr = self.map_address(address);
        if CoreMemory::is_special_addr(mapped_addr) {
            if let Some(listener) = self.listeners.get(&address) {
                return listener.borrow_mut().read(self, mapped_addr);
            }
        }

        self.memory[mapped_addr as usize]
    }

    pub fn read16(&self, address: u16) -> u16 {
        let mapped_addr = self.map_address(address);
        /* TODO HACK: speed up memory access by only looking for listeners on a small number
         * of whitelisted addresses; will have to revisit this
         */
        if CoreMemory::is_special_addr(mapped_addr) {
            if self.listeners.get(&address).is_some() {
                panic!("read16 not supported for listened-to addresses");
            }
        }

        let lo_byte = self.memory[mapped_addr as usize] as u16;
        let hi_byte = self.memory[(mapped_addr+1) as usize] as u16;

        lo_byte | (hi_byte << 8)
    }

    pub fn read_no_listen(&self, address: u16) -> u8 {
        self.memory[self.map_address(address) as usize]
    }

    /* NB: this does not activate listeners! */
    pub fn read_slice(&self, address: u16, size: usize) -> &[u8]{
        let mapped_addr = self.map_address(address) as usize;

        &self.memory[mapped_addr..mapped_addr + size]
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let mapped_addr = self.map_address(address);
        /* TODO HACK: speed up memory access by only looking for listeners on a small number
         * of whitelisted addresses; will have to revisit this
         */
        if CoreMemory::is_special_addr(mapped_addr) {
            if let Some(listener) = self.listeners.get(&mapped_addr) {
                listener.borrow_mut().write(self, mapped_addr, value);
            }
        }

        /* note that even if there's a listener, it still writes like normal */
        self.memory[mapped_addr as usize] = value;
    }

    pub fn is_special_addr(address: u16) -> bool {
        let ppu_reg = 0x2000 <= address && address < 0x2008;
        let apu_io = 0x4000 <= address && address < 0x4018;

        ppu_reg || apu_io
    }

    pub fn set_nmi(&mut self, nmi_set: bool) {
        self.nmi_flag = nmi_set;
    }

    pub fn nmi_set(&self) -> bool {
        self.nmi_flag
    }

    pub fn register_listener(&mut self, listener: Rc<RefCell<dyn MemoryListener>>) {
        for addr in listener.borrow().get_addresses() {
            if self.listeners.get(&addr).is_some() {
                panic!("Attempting to register a second memory listener at address 0x{addr:x}");
            }
            self.listeners.insert(addr, listener.clone());
        }
    }

    fn map_address(&self, addr: u16) -> u16 {
        if addr > 0x7ff && addr <= 0x1fff {
            addr & 0x7ff
        } else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
            0x2000 | (addr & 0x7)
        } else {
            addr
        }
    }
}