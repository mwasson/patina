use std::collections::HashSet;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder};
use crate::ppu::{WriteBuffer};

pub fn initialize_ui(write_buffer : Arc<Mutex<WriteBuffer>>, keys : Arc<Mutex<HashSet<VirtualKeyCode>>>) -> Result<(), Box<dyn std::error::Error>> {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Patina")
		.with_inner_size(LogicalSize::new(276, 256))
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

				frame.copy_from_slice(write_buffer.lock().unwrap().deref());

				let _ = pixels.render();
			}
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::CloseRequested => {
					*control_flow = ControlFlow::Exit;
				}
				WindowEvent::Resized(size) => {
					pixels.resize_surface(size.width, size.height).expect("TODO: panic message");
				}
				WindowEvent::KeyboardInput {
					input, ..
				} => {
					match input.state {
					    ElementState::Pressed => {
							if let Some(key) = input.virtual_keycode {
								keys.lock().unwrap().insert(key);
							}
						},
						ElementState::Released => {
							if let Some(key) =  input.virtual_keycode {
								keys.lock().unwrap().remove(&key);
							}
						}
					}
				}
				_ => ()
			}
			_ => ()
		}
	});
}