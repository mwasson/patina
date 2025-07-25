use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use crate::cpu::{MEMORY_SIZE};
use crate::ppu::{PPUListener, PPURegister};
use crate::read_write::ReadWrite;
/* TODO: should this also handle side-effects? */

#[derive(Clone)]
pub struct CoreMemory
{
    address_mapper: fn(u16) -> Option<u16>,
    internals: Arc<Mutex<CoreMemoryInternals>>,
    listener: Option<PPUListener>,
}

struct CoreMemoryInternals
{
    memory: [u8; MEMORY_SIZE],
    nmi_triggered: bool,
}

impl CoreMemory {
    pub fn write(&mut self, addr: u16, data: u8) {
        self.internals.lock().unwrap().memory[self.map_address(addr)] = data;
        self.check_for_listener(addr, ReadWrite::WRITE, data);
        if(addr >= 0x300 && addr < 0x700 && data == 0xaa) {
            println!("writing 0xaa to: 0x{addr:x}");
            println!();
            if(addr == 0x42e) {
                /* program counter 0x871c */
                self.print_memory(0x8700,0x100);
                println!("break here");
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let mut result = self.internals.lock().unwrap().memory[self.map_address(addr)];
        /* in some cases, the listener can modify the results */
        let listener_result = self.check_for_listener(addr, ReadWrite::READ, result);
        if listener_result.is_some() {
            self.internals.lock().unwrap().memory[self.map_address(addr)] = listener_result.unwrap();
            return listener_result.unwrap()
        }
        result
    }

    pub fn copy_range(&self, base_addr: usize, dst: &mut [u8]) {
        let mapped_base_addr = self.map_address(base_addr as u16);
        dst.copy_from_slice(&self.internals.lock().unwrap().memory[mapped_base_addr..(mapped_base_addr+dst.len())]);
    }

    fn map_address(&self, addr: u16) -> usize {
        (self.address_mapper)(addr).unwrap_or(addr) as usize
    }

    /* TODO: might want to delete this, this was just to get an idea */
    pub fn clone(&self) -> CoreMemory {
        CoreMemory {
            address_mapper: self.address_mapper,
            internals: self.internals.clone(),
            listener: self.listener.clone(),
        }
    }

    pub fn new(memory: [u8; MEMORY_SIZE]) -> CoreMemory {
        CoreMemory {
            address_mapper: CoreMemory::ppu_mirror(),
            internals: Arc::new(Mutex::new(CoreMemoryInternals{
                memory,
                nmi_triggered: false,
            })),
            listener: None
        }
    }

    pub fn register_listener(&mut self, listener: PPUListener) {
        println!("Registering listener!");
        self.listener = Some(listener);
    }

    pub fn nmi_triggered(&self) -> bool {
        self.internals.lock().unwrap().nmi_triggered
    }

    pub fn trigger_nmi(&mut self) {
        self.internals.lock().unwrap().nmi_triggered = true;
    }

    pub fn reset_nmi(&mut self) {
        self.internals.lock().unwrap().nmi_triggered = false;
    }

    /* for debugging */
    pub fn print_memory(&self, base_addr: usize, len: usize) {
        let mut mem = Vec::new();
        mem.extend_from_slice(&self.internals.lock().unwrap().memory[base_addr..(base_addr+len)]);
        let mut output = String::new();
        output.push_str(format!("Memory [0x{:x}..0x{:x}] : \n", base_addr, base_addr+len).as_str());
        for i in 0..len {
            output.push_str(format!("[0x{:x}]: 0x{:x}\n", base_addr+i, mem[i]).as_str());
        }
        println!("{}", output);
    }

    /* the PPU registers are at 0x2000 through 0x2007; they're then remapped every eight bytes up
     * through 0x3fff. This reflects that.
     */
    fn ppu_mirror() -> fn(u16) -> Option<u16> {
        |addr:u16| -> Option<u16> {
            if addr > 0x7ff && addr <= 0x1fff {
                println!("remapping memory mirror: 0x{:x} to 0x{:x}", addr, addr & 0x7ff);
                return Some(addr & 0x7ff);
            }

            if addr <= 0x1fff { /* system memory */
                return Some(addr & 0x7FF);
            }  else if addr >= 0x2000 && addr <= 0x3FFF { /* ppu registers */
                return Some(0x2000 | (addr & 0x7));
            } else {
                return None;
            }
        }
    }

    fn check_for_listener(&self, addr:u16, read_write: ReadWrite, value: u8) -> Option<u8> {
        if self.listener.is_some() {
            let possible_register = PPURegister::from_addr(addr);
            if possible_register.is_some() {
                let register = possible_register.unwrap();
                return self.listener.clone().unwrap().listen(&self, register, read_write, value);
            }
        }
        None
    }
}