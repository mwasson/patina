use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use crate::cpu::MEMORY_SIZE;
use crate::rom::Rom;

pub trait MemoryListener {
    fn get_addresses(&self) -> Vec<u16>;
    fn read(&mut self, memory: &CoreMemory, address: u16) -> u8;
    fn write(&mut self, memory: &CoreMemory, address: u16, value: u8);
}

pub struct CoreMemory {
    memory: [u8; MEMORY_SIZE],
    listeners: HashMap<u16, Arc<RwLock<dyn MemoryListener>>>
}

impl CoreMemory {
    pub fn new(rom: &Rom) -> CoreMemory {
        /* TODO: handling RAM, mappers, etc. */
        let mut memory = [0; MEMORY_SIZE];
        memory[(0x10000 - rom.prg_data.len())..0x10000].copy_from_slice(&*rom.prg_data);

        CoreMemory {
            memory,
            listeners: HashMap::new()
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        let mapped_addr = self.map_address(address);
        if let Some(listener) = self.listeners.get(&address) {
            listener.write().unwrap().read(self, mapped_addr)
        } else {
            self.read_no_listen(mapped_addr)
        }
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
        if let Some(listener) = self.listeners.get(&mapped_addr) {
            listener.write().unwrap().write(self, mapped_addr, value);
        }

        /* note that even if there's a listener, it still writes like normal */
        self.memory[mapped_addr as usize] = value;
    }

    pub fn register_listener(&mut self, listener: Arc<RwLock<dyn MemoryListener>>) {
        for addr in listener.read().unwrap().get_addresses() {
            if self.listeners.get(&addr).is_some() {
                panic!("Attempting to register a second memory listener at address 0x{addr:x}");
            }
            self.listeners.insert(addr, listener.clone());
        }
    }


    fn map_address(&self, addr: u16) -> u16 {
        let mapped_addr = if addr > 0x7ff && addr <= 0x1fff {
            addr & 0x7ff
        } else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
            0x2000 | (addr & 0x7)
        } else {
            addr
        };

        mapped_addr
    }
}