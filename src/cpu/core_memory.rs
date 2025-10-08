use crate::cpu::{Controller, CPU, MEMORY_SIZE};
use crate::mapper::Mapper;
use crate::rom::Rom;
use fnv::FnvHashMap;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use crate::apu::APU;
use crate::ppu::PPU;

pub struct SharedItems<'a> {
    pub(crate) apu: &'a mut APU,
    pub(crate) ppu: &'a mut PPU,
    pub(crate) mapper: &'a mut dyn Mapper,
}

pub trait MemoryListener {
    fn get_addresses(&self) -> Vec<u16>;

    fn read(&self, memory: &CoreMemory, shared_items: &mut SharedItems, address: u16) -> u8;
    fn write(&self, memory: &CoreMemory, shared_items: &mut SharedItems, address: u16, value: u8);
}

pub struct CoreMemory {
    memory: Box<[u8; MEMORY_SIZE]>,
    nmi_flag: bool, /* a convenience, to avoid a PPU dependency on the CPU */
    listeners: FnvHashMap<u16, Rc<dyn MemoryListener>>,
}

impl CoreMemory {
    pub fn new(rom: &Rom) -> CoreMemory {
        CoreMemory {
            memory: Box::new([0; MEMORY_SIZE]),
            nmi_flag: false,
            listeners: FnvHashMap::with_capacity_and_hasher(10, Default::default()),
        }
    }

    pub fn read(&self, shared_items: &mut SharedItems, address: u16) -> u8 {
        let mapped_addr = self.map_address(address);
        if CoreMemory::is_special_addr(mapped_addr) {
            if let Some(listener) = self.listeners.get(&mapped_addr) {
                return listener.read(self, shared_items, mapped_addr);
            }
        }
        self.read_no_listen_no_map(shared_items.mapper, mapped_addr)
    }

    pub fn read16(&self, mapper: &dyn Mapper, address: u16) -> u16 {
        let mapped_addr = self.map_address(address);
        /* TODO HACK: speed up memory access by only looking for listeners on a small number
         * of whitelisted addresses; will have to revisit this
         */
        if CoreMemory::is_special_addr(mapped_addr) {
            if self.listeners.get(&address).is_some() {
                panic!("read16 not supported for listened-to addresses");
            }
        }

        /* there's a bug where if there's a page crossing, it reads from the bottom of the same
         * page */
        let in_page_addr = (mapped_addr % 256) as u8;
        let page_base = mapped_addr & !0xff;
        let hi_byte_addr = page_base + in_page_addr.wrapping_add(1) as u16;
        let lo_byte = self.read_no_listen_no_map(mapper, mapped_addr) as u16;
        let hi_byte = self.read_no_listen_no_map(mapper, hi_byte_addr) as u16;

        lo_byte | (hi_byte << 8)
    }

    fn read_no_listen_no_map(&self, mapper: &dyn Mapper, address: u16) -> u8 {
        /* high addresses go to the on-cartridge mapper */
        if address >= 0x4020 {
            mapper.read_prg(address)
        /* low addresses handled by on-board memory */
        } else {
            self.memory[address as usize]
        }
    }

    /* NB: this does not activate listeners! */
    pub fn copy_slice(&self, mapper: &dyn Mapper, address: u16, size: usize, dest: &mut [u8]) {
        let mapped_addr = self.map_address(address) as usize;
        if mapped_addr < 0x4000 {
            dest.copy_from_slice(&self.memory[mapped_addr..mapped_addr + size]);
        } else {
            dest.copy_from_slice(&mapper.read_prg_slice(address, size));
        }
    }

    pub fn write(&mut self, shared_items: &mut SharedItems, address: u16, value: u8) {
        /* addresses that appear to be control registers for the Famicom Disk System; ignore */
        if address >= 0x4020 && address < 0x4100 {
            return;
        }
        /* high addresses go to the on-cartridge mapper */
        if address >= 0x4100 {
            shared_items.mapper.write_prg(address, value);
        /* low addresses handled by on-board memory */
        } else {
            let mapped_addr = self.map_address(address);
            /* TODO HACK: speed up memory access by only looking for listeners on a small number
             * of whitelisted addresses; will have to revisit this
             */
            if CoreMemory::is_special_addr(mapped_addr) {
                if let Some(listener) = self.listeners.get(&mapped_addr) {
                    listener.write(self, shared_items, mapped_addr, value);
                    return;
                }
            }

            self.memory[mapped_addr as usize] = value;
        }
    }

    pub fn is_special_addr(address: u16) -> bool {
        let ppu_reg = 0x2000 <= address && address < 0x2008;
        let apu_io = 0x4000 <= address && address < 0x4018;

        ppu_reg || apu_io
    }

    pub fn register_listener(&mut self, listener: Rc<dyn MemoryListener>) {
        for addr in listener.get_addresses() {
            if self.listeners.get(&addr).is_some() {
                panic!("Attempting to register a second memory listener at address 0x{addr:x}");
            }
            self.listeners.insert(addr, listener.clone());
        }
    }

    fn map_address(&self, addr: u16) -> u16 {
        if addr <= 0x3fff {
            if addr > 0x7ff && addr <= 0x1fff {
                addr & 0x7ff
            } else if addr > 0x1fff {
                /* ppu registers */
                0x2000 | (addr & 0x7)
            } else {
                addr
            }
        } else {
            addr
        }
    }

    pub fn open_bus(&self) -> u8 {
        /* TODO not how hardware behaves */
        0
    }
}
