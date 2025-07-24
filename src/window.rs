use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use crate::ppu::{PPUState, WriteBuffer};

pub fn initialize_ui(write_buffer : Arc<Mutex<WriteBuffer>>) -> Result<(), Box<dyn std::error::Error>> {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Patina")
		.with_inner_size(LogicalSize::new(512, 256))
		.build(&event_loop)
		.unwrap();

	let mut pixels = {
		let window_size = window.inner_size();
		let surface_texture = SurfaceTexture::new(window_size.width,
												  window_size.height,
												  &window);
		Pixels::new(256, 240, surface_texture)?
	};

	thread::spawn(move || {
		loop {
			thread::sleep(Duration::from_millis(1000/60));
			window.request_redraw();
		}
	});

	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::RedrawRequested(_) => {
				let frame = pixels.frame_mut();

				/* clear screen */
				for pixel in frame.chunks_exact_mut(4) {
					pixel.copy_from_slice(&[0, 0, 0, 255]);
				}

				frame.copy_from_slice(write_buffer.lock().unwrap().deref());

				// draw_circle(frame, 640 / 2, 480 / 2, 100);

				let _ = pixels.render();
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