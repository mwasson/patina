use crate::apu::APU;
use crate::cpu::{CoreMemory, CPU};
use crate::mapper::Mapper;
use crate::ppu::ppu_listener::PPUListener;
use crate::ppu::{WriteBuffer, PPU, WRITE_BUFFER_SIZE};
use crate::rom::Rom;
use crate::simulator::scheduler::Scheduler;
use crate::simulator::SimulatorSignal;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fs, thread};
use winit::keyboard::Key;

/**
 * Provides an external view into a running emulator state. This packages up all relevant parts
 * of the emulation (CPU, PPU, memory, etc.) and runs them on a different thread. This handles
 * all thread communication, as well as joining when complete. It all handles management of
 * external resources.
 */
pub struct ProgramState {
    /* inputs */
    pub key_source: Arc<Mutex<HashSet<Key>>>,

    /* outputs */
    pub write_buffer: Arc<Mutex<WriteBuffer>>,
    pub thread_handle: Option<JoinHandle<Option<Vec<u8>>>>,

    /* communication */
    thread_sender: Sender<SimulatorSignal>,
}

impl ProgramState {
    pub fn simulate_async(
        rom: &Rom,
        savefile: &Option<String>,
        key_source: Arc<Mutex<HashSet<Key>>>,
    ) -> ProgramState {
        let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));
        let mapper = rom.initialize_mapper();

        let (thread_sender, thread_receiver) = channel::<SimulatorSignal>();

        let mut result = ProgramState {
            key_source,
            write_buffer,
            thread_sender,
            thread_handle: None,
        };

        result.simulate_async_internal(mapper, savefile, thread_receiver);

        result
    }

    fn simulate_async_internal(
        &mut self,
        mapper: Box<dyn Mapper>,
        savefile: &Option<String>,
        thread_receiver: Receiver<SimulatorSignal>,
    ) {
        let write_buffer = self.write_buffer.clone();
        let key_source_clone = self.key_source.clone();
        let savefile = savefile.clone();

        self.thread_handle = Some(thread::spawn(move || {
            let mut memory = Box::new(CoreMemory::new_from_mapper(mapper));

            let ppu = PPU::new(write_buffer, memory.mapper.clone());

            let apu = APU::new();
            memory.register_listener(apu.clone());

            let ppu_listener = PPUListener::new(ppu.clone());
            memory.register_listener(Rc::new(RefCell::new(ppu_listener)));

            let mut cpu = CPU::new(memory);
            cpu.set_key_source(key_source_clone);

            if let Some(save_data) = Self::load_save_data(&savefile) {
                cpu.set_save_data(&save_data);
            }

            let mut scheduler = Scheduler::new(cpu, ppu, apu, thread_receiver);

            scheduler.simulate()
        }));
    }

    pub fn cleanup(&mut self) -> Option<Vec<u8>> {
        if let Some(thread_handle) = self.thread_handle.take() {
            self.thread_sender
                .send(SimulatorSignal::EndSimulation)
                .expect("Could not send EndSimulation");

            match thread_handle.join() {
                Ok(save_data) => save_data,
                Err(x) => {
                    panic!("Unexpected panic on join to stop emulation: {:?}", x);
                }
            }
        } else {
            None
        }
    }

    fn load_save_data(savefile: &Option<String>) -> Option<Vec<u8>> {
        match savefile {
            None => None,
            Some(path) => {
                /* TODO better error handling */
                fs::read(path).ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom::Rom;
    use std::sync::Mutex;

    fn make_test_rom() -> Rom {
        Rom {
            prg_data: vec![0u8; 16384],
            chr_data: vec![0u8; 8192],
            byte_6_flags: 0,
            byte_7_flags: 0,
            _trainer: vec![],
            _prg_ram: vec![],
            _tv_system: 0,
        }
    }

    #[test]
    fn simulate_async_starts_thread_and_cleanup_stops_it() {
        let keys = Arc::new(Mutex::new(HashSet::new()));
        let mut state = ProgramState::simulate_async(&make_test_rom(), &None, keys);
        assert!(state.thread_handle.is_some());
        assert!(state.cleanup().is_none());
        assert!(state.thread_handle.is_none());
    }
}
