use std::{env, thread};
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::io::{self, ErrorKind};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

mod cpu;

mod rom;
use rom::Rom;
use crate::apu::APU;
use crate::cpu::{CoreMemory, CPU};
use crate::ppu::{PPU, WRITE_BUFFER_SIZE};
use crate::ppu::ppu_listener::PPUListener;
use scheduler::RenderRequester;

mod window;
mod ppu;
mod processor;
mod scheduler;
mod apu;

fn main() -> Result<(), Box<dyn Error>> {
	println!("Here begins the Patina project. An inauspicious start?");
	let args = env::args().collect::<Vec<_>>();

	let rom = if let Some(rom_path) = args.get(1) {
		let path = &*rom_path.clone();
		Rom::parse_file(path)?
	} else {
		return Err(Box::new(io::Error::new(ErrorKind::Other, "First argument must be ROM file path")));
	};

	let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));
	let write_buffer_clone = write_buffer.clone();

	let keys = Arc::new(Mutex::new(HashSet::new()));
	let keys_clone = keys.clone();
	let render_listener = Arc::new(Mutex::new(RenderRequester::new()));
	let render_listener_clone = render_listener.clone();
	
	thread::spawn(move || {
		let memory = Rc::new(RefCell::new(CoreMemory::new(&rom)));
		let ppu = PPU::from_rom(&rom, write_buffer_clone, memory.clone());
		let mut cpu = CPU::new(memory.clone());
		
		let apu = APU::new(memory.clone());
		memory.borrow_mut().register_listener(apu.clone());

		let ppu_listener = PPUListener::new(ppu.clone());
		memory.borrow_mut().register_listener(Rc::new(RefCell::new(ppu_listener)));

		cpu.set_key_source(keys_clone);
		scheduler::simulate(&mut cpu, ppu, apu, render_listener_clone);
	});

	match window::initialize_ui(write_buffer, keys, render_listener) {
	    Ok(()) => Ok(()),
		Err(event_loop_error) => Err(event_loop_error.into())
	}
}