use std::fs;
use std::io::{self, ErrorKind};

use pixels::{Pixels, SurfaceTexture};

use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod rom;
use rom::Rom;

fn main() -> Result<(), pixels::Error> {
	println!("Here begins the Patina project. An inauspicious start?");
	parse_file("fileloc");
	
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Patina")
		.with_inner_size(LogicalSize::new(640 as f64, 480 as f64))
		.build(&event_loop)
		.unwrap();

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width,
		                                          window_size.height,
		                                          &window);
		Pixels::new(640, 480, surface_texture)?
	};

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::RedrawRequested(_) => {
				let frame = pixels.frame_mut();

				/* clear screen */
				for pixel in frame.chunks_exact_mut(4) {
					pixel.copy_from_slice(&[0,0,0,255]);
				}
				
				draw_circle(frame, 640 / 2, 480 /2, 100);

				pixels.render();
			}
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::CloseRequested => {
					*control_flow = ControlFlow::Exit;
				}
				_ => ()
			}
			_ => ()
		}
	});
}

fn draw_circle(frame: &mut [u8], center_x: i32, center_y: i32, radius: i32) {
	for y in -radius..radius {
		for x in -radius..radius {
			if x*x + y*y <= radius*radius {
				let mx = center_x + x;
				let my = center_y + y;

				if mx >= 0 && mx < 640 && my >= 0 && my < 480 {
					let loc = (my*640 + mx) as usize * 4;
					frame[loc] = 0;
					frame[loc+1] = 0;
					frame[loc+2] = 255;
					frame[loc+3] = 255;	
				}
			}
		} 
	}
}

fn parse_file(file_ref: &str) -> io::Result<Vec<u8>> {
	let rom_data: Vec<u8> = fs::read(file_ref)?;
	validate_header(&rom_data);

	return Ok(rom_data);
}


/* TODO: Result should probably be std Result, not io Result */
fn validate_header(rom_data: &Vec<u8>) -> io::Result<()> {
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
		return Err(io::Error::new(ErrorKind::InvalidData, error_msg));	
	} else {
		return Ok(());
	}
}
