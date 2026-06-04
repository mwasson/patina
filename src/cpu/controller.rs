use crate::cpu::core_memory::MemoryListener;
use crate::cpu::CoreMemory;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tao::keyboard::Key;

pub const CONTROLLER_ADDRESS: u16 = 0x4016;

#[derive(Clone)]
pub struct Controller {
    key_source: Arc<Mutex<HashSet<Key<'static>>>>,
    inputs_in_order: Vec<u8>,
    old_value: u8,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            key_source: Arc::new(Mutex::new(HashSet::new())), /* will be overwritten, that's fine */
            inputs_in_order: Vec::new(),
            old_value: 0,
        }
    }

    pub fn set_key_source(&mut self, keys: Arc<Mutex<HashSet<Key<'static>>>>) {
        self.key_source = keys;
    }

    pub fn record_data(&mut self) {
        let recorded_keys = self.key_source.lock().unwrap().clone();
        self.inputs_in_order = Vec::new();
        /* keys, in order: A B Select Start Up Down Left Right */
        /* putting in a stack, so will reverse it */
        self.push_key_press(&recorded_keys, Key::ArrowRight); /* right */
        self.push_key_press(&recorded_keys, Key::ArrowLeft); /* left */
        self.push_key_press(&recorded_keys, Key::ArrowDown); /* down */
        self.push_key_press(&recorded_keys, Key::ArrowUp); /* up */
        self.push_key_press(&recorded_keys, Key::Enter); /* start */
        self.push_key_press(&recorded_keys, Key::Tab); /* select */
        self.push_key_press(&recorded_keys, Key::Character("z")); /* B */
        self.push_key_press(&recorded_keys, Key::Character("x"));
        /* A */
    }

    pub fn get_next_byte(&mut self) -> u8 {
        self.inputs_in_order.pop().unwrap_or(1)
    }

    fn push_key_press(&mut self, recorded_keys: &HashSet<Key<'static>>, key: Key<'static>) {
        self.inputs_in_order
            .push(if recorded_keys.contains(&key) { 1 } else { 0 })
    }
}

impl MemoryListener for Controller {
    fn get_addresses(&self) -> Vec<u16> {
        let mut addrs = Vec::new();

        addrs.push(CONTROLLER_ADDRESS);

        addrs
    }

    fn read(&mut self, _memory: &CoreMemory, _address: u16) -> u8 {
        self.get_next_byte()
    }

    fn write(&mut self, _memory: &CoreMemory, _address: u16, value: u8) {
        if self.old_value & 1 == 1 && value & 1 == 0 {
            self.record_data();
        }
        self.old_value = value;
    }
}
