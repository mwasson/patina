use std::{fs, io};
use std::io::ErrorKind;
use crate::ppu::{Tile, index_to_pixel};

pub struct Rom {
    pub prg_data: Vec<u8>,
    pub chr_data: Vec<u8>,
    pub byte_6_flags: u8, /* TODO: split these out */
    pub byte_7_flags: u8, /* TODO: split these out */
    pub trainer: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub tv_system: u8, /* TODO: make into a boolean or enum */
}

impl Rom {
    /* TODO this stuff should *all* be somewhere else */
    pub fn render(&self, write_buffer: &mut [u8], width: usize) {
        Rom::render_pattern_table(&self.chr_data[0..(256 * 16)], write_buffer, width, 0);
        Rom::render_pattern_table(&self.chr_data[(256 * 16)..(256 * 32)], write_buffer, width, 128);
    }

    pub fn render_pattern_table(pattern_table: &[u8], write_buffer: &mut [u8], width: usize, start_x: usize) {
        for i in (0..256) {
            let xy = index_to_pixel(16, i);
            let data_start = i * 16;
            let mut tile_data = [0 as u8; 16];
            tile_data.copy_from_slice(&pattern_table[data_start..data_start + 16]);
            let tile = Tile::from_memory(tile_data);
            tile.stamp(write_buffer, width, xy.0 * 8 + start_x, xy.1 * 8);
        }
    }
    pub fn parse_file(file_ref: &str) -> io::Result<Rom> {
        println!("Attempting to parse {}", file_ref);
        let rom_data: Vec<u8> = fs::read(file_ref)?;
        Rom::read_rom_data(&rom_data)
    }


    /* TODO: Result should probably be std Result, not io Result */
    fn read_rom_data(rom_data: &Vec<u8>) -> io::Result<Rom> {
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
}
