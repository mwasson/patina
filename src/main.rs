use std::fs;
use std::io::{self, Error, ErrorKind};

fn main() {
    println!("Here begins the Patina project. An inauspicious start?");
    parse_file("fileloc");
}

fn parse_file(file_ref: &str) -> io::Result<Vec<u8>> {
	let rom_data: Vec<u8> = fs::read(file_ref)?;
	validate_header(&rom_data);

	return Ok(rom_data);
}


fn validate_header(rom_data: &Vec<u8>) -> Result<(), io::Error> {
	let mut error_msg = String::from("");

	if rom_data.len() < 16 {
		error_msg = String::from("The ROM must be at least 16 bytes long.");
	} 

	let header_data = &rom_data[0..4];
	if header_data != b"NES\x1A" {
		error_msg = format!("The ROM's header must meet the NES ROM specification; however, it was: {:?}", header_data);
	}

	/* parse section sizes; PRG ROM is in 16k increments,
	 * CHR ROM is in 8k (and can be zero) */
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
		prg_rom: rom_data[prg_rom_start..chr_rom_start].to_vec(),
		chr_ram: (&rom_data[chr_rom_start..chr_rom_start+chr_rom_size]).to_vec(),
		byte_6_flags: rom_data[6],
		byte_7_flags: rom_data[7],
		trainer: vec![], /* TODO */
		prg_ram: vec![], /* TODO */
		tv_system: rom_data[9]
	};

	if error_msg != "" {
		return Err(Error::new(ErrorKind::InvalidData, error_msg));	
	} else {
		return Ok(());
	}
}

/* TODO: move into separate file */
struct Rom {
	prg_rom: Vec<u8>,
	chr_ram: Vec<u8>,
	byte_6_flags: u8, /* TODO: split these out */
	byte_7_flags: u8, /* TODO: split these out */
	trainer: Vec<u8>,
	prg_ram: Vec<u8>,
	tv_system: u8, /* TODO: make into a boolean or enum */
}
