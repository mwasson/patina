use std::{env, fs, thread};
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
		parse_file(path)?
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
		let mut cpu = CPU::from_rom(&rom, memory.clone());
		let mut apu = Box::new(APU::new(memory.clone()));

		let ppu_listener = PPUListener::new(ppu.clone());
		memory.clone().borrow_mut().register_listener(Rc::new(RefCell::new(ppu_listener)));

		cpu.set_key_source(keys_clone);
		scheduler::simulate(&mut cpu, ppu, &mut apu, render_listener_clone);
	});

	match window::initialize_ui(write_buffer, keys, render_listener) {
	    Ok(()) => Ok(()),
		Err(eventLoopError) => Err(eventLoopError.into())
	}
}

fn parse_file(file_ref: &str) -> io::Result<Rom> {
	println!("Attempting to parse {}", file_ref);
	let rom_data: Vec<u8> = fs::read(file_ref)?;
	validate_header(&rom_data)
}


/* TODO: Result should probably be std Result, not io Result */
fn validate_header(rom_data: &Vec<u8>) -> io::Result<Rom> {
	println!("ROM validation...");
	let mut error_msg = String::from("");

	if rom_data.len() < 16 {
		error_msg = String::from("The ROM must be at least 16 bytes long.");
	}

	let header_data = &rom_data[0..4];
	if header_data != b"NES\x1A" {
		error_msg = format!("The ROM's header must meet the NES ROM specification; however, it was: {:?}", header_data);
	}

	/* parse section sizes; PRG ROM is in 16k increments,
	 * CHR ROM is in 8k (and can be zero) TODO: this does not handle that case */
	let prg_rom_size = (rom_data[4] as usize) * (1 << 14 /*16k*/);
	let chr_rom_size = (rom_data[5] as usize) * (1 << 13 /*8k*/);

	/* todo: assert bytes 10-15 are zero */

	/* TODO: read trainer */

	/* TODO modify to include trainer */
	let prg_rom_start = 16;
	let chr_rom_start = prg_rom_start + prg_rom_size;

	/* TODO: how does the prg ram work? */

	/* TODO: This is not the correct data yet */
	/* TODO: Would it be better to use Cow here? */
	let rom = Rom {
		prg_data: (&rom_data[prg_rom_start..chr_rom_start]).to_vec(),
		chr_data: (&rom_data[chr_rom_start..chr_rom_start + chr_rom_size]).to_vec(),
		byte_6_flags: rom_data[6],
		byte_7_flags: rom_data[7],
		trainer: vec![], /* TODO */
		prg_ram: vec![], /* TODO */
		tv_system: rom_data[9]
	};

	println!("Rom flags:");
	println!("{}", rom.byte_6_flags);
	println!("{}", rom.byte_7_flags);
	println!("PRG size: {}", rom.prg_data.len());
	println!("CHR size: {}", rom.chr_data.len());

	if error_msg != "" {
		Err(io::Error::new(ErrorKind::InvalidData, error_msg))
	} else {
		Ok(rom)
	}
}