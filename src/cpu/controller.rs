use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use winit::event::VirtualKeyCode;

#[derive(Clone)]
pub struct Controller {
    key_source: Arc<Mutex<HashSet<VirtualKeyCode>>>,
    inputs_in_order: Vec<u8>
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            key_source: Arc::new(Mutex::new(HashSet::new())), /* will be overwritten, that's fine */
            inputs_in_order: Vec::new(),
        }
    }

    pub fn set_key_source(&mut self, keys: Arc<Mutex<HashSet<VirtualKeyCode>>>) {
        self.key_source = keys;
    }

    pub fn record_data(&mut self) {
        let recorded_keys = self.key_source.lock().unwrap().clone();
        self.inputs_in_order = Vec::new();
        /* keys, in order: A B Start Select Up Down Left Right */
        /* putting in a stack, so will reverse it */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Right); /* right */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Left); /* left */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Up); /* down */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Down); /* up */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Tab); /* select */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Return); /* start */
        self.push_key_press(&recorded_keys, VirtualKeyCode::Z); /* B */
        self.push_key_press(&recorded_keys, VirtualKeyCode::X); /* A */
    }

    pub fn get_next_byte(&mut self) -> u8 {
        self.inputs_in_order.pop().unwrap_or(1)
    }

    fn push_key_press(&mut self, recorded_keys: &HashSet<VirtualKeyCode>, key: VirtualKeyCode) {
        self.inputs_in_order.push(if recorded_keys.contains(&key) { 1 } else { 0})
    }
}