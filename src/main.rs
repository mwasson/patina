use std::{fs, thread};
use std::io::{self, ErrorKind};
use std::sync::mpsc::channel;
use std::time::Instant;

mod cpu;

mod rom;
use rom::Rom;
use crate::cpu::{ProgramState};
use crate::ppu::PPUState;

mod window;
mod ppu;
mod processor;
mod scheduler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Here begins the Patina project. An inauspicious start?");
	let rom = parse_file("/Users/mwasson/smb.nes")?; /* temporary, for testing */

	let (nmi_sender, nmi_receiver) = channel();
	let (update_sender, update_receiver) = channel();

	let mut cpu = ProgramState::from_rom(&rom, nmi_receiver, update_sender);
	let mut ppu = PPUState::from_rom(&rom, nmi_sender, update_receiver);
	
	let write_buffer = ppu.get_write_buffer();

	thread::spawn(move || {
		scheduler::simulate(&mut cpu, &mut ppu);
	});

	// TODO: link PPU to window
	window::initialize_ui(write_buffer)
}

fn parse_file(file_ref: &str) -> io::Result<Rom> {
	println!("Attempting to parse {}", file_ref);
	let rom_data: Vec<u8> = fs::read(file_ref)?;
	return validate_header(&rom_data);
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
		return Err(io::Error::new(ErrorKind::InvalidData, error_msg));
	} else {
		return Ok(rom);
	}
}