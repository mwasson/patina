use crate::apu::APU;
use crate::cpu::{CoreMemory, CPU};
use crate::mapper::Mapper;
use crate::ppu::ppu_listener::PPUListener;
use crate::ppu::{WriteBuffer, PPU, WRITE_BUFFER_SIZE};
use crate::rom::Rom;
use crate::scheduler;
use crate::scheduler::RenderRequester;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use winit::keyboard::Key;

/**
 * Provides an external view into a running emulator state. This packages up all relevant parts
 * of the emulation (CPU, PPU, memory, etc.) and runs them on a different thread. This handles
 * all thread communication, as well as joining when complete. It all handles management of
 * external resources.
 */
pub struct ProgramState {
    /* inputs */
    pub render_requester: Arc<Mutex<RenderRequester>>,
    pub key_source: Arc<Mutex<HashSet<Key>>>,

    /* outputs */
    pub write_buffer: Arc<Mutex<WriteBuffer>>,
    pub thread_handle: Option<JoinHandle<()>>,
}

impl ProgramState {
    pub fn simulate_async(rom: &Rom, key_source: Arc<Mutex<HashSet<Key>>>) -> ProgramState {
        let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));
        let mapper = rom.initialize_mapper();

        let render_requester = Arc::new(Mutex::new(RenderRequester::new()));

        let mut result = ProgramState {
            key_source,
            write_buffer,
            render_requester,
            thread_handle: None,
        };

        result.simulate_async_internal(mapper);

        result
    }

    fn simulate_async_internal(&mut self, mapper: Box<dyn Mapper>) {
        let write_buffer = self.write_buffer.clone();
        let render_requester = self.render_requester.clone();
        let key_source_clone = self.key_source.clone();

        self.thread_handle = Some(thread::spawn(move || {
            let mut memory = Box::new(CoreMemory::new_from_mapper(mapper));

            let ppu = PPU::new(write_buffer, memory.mapper.clone(), render_requester);

            let apu = APU::new();
            memory.register_listener(apu.clone());

            let ppu_listener = PPUListener::new(ppu.clone());
            memory.register_listener(Rc::new(RefCell::new(ppu_listener)));

            let mut cpu = CPU::new(memory);
            cpu.set_key_source(key_source_clone);

            scheduler::simulate(&mut cpu, ppu, apu);
        }));
    }

    // next stage: thread cleanup
    // pub fn cleanup(&mut self) {
    //     if let Some(thread_handle) = self.thread_handle.take() {
    //         /* TODO: send message to stop emulation */
    //
    //         match thread_handle.join() {
    //             Ok(_) => {}
    //             Err(x) => {
    //                 panic!("Unexpected panic on join to stop emulation: {:?}", x);
    //             }
    //         }
    //     }
    // }
}
