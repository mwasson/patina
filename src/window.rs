use std::collections::HashSet;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::Key;
use winit::window::{Window, WindowId};
use crate::ppu::{WriteBuffer};

struct WindowApp<'a> {
	write_buffer : Arc<Mutex<WriteBuffer>>,
	keys: Arc<Mutex<HashSet<Key>>>,
	pixels: Option<Pixels<'a>>,
	window: Option<Arc<Window>>,
}

impl WindowApp<'_> {
	fn new(write_buffer : Arc<Mutex<WriteBuffer>>, keys: Arc<Mutex<HashSet<Key>>>) -> Self {
		Self {
			write_buffer,
			keys,
			pixels: None,
			window: None,
		}
	}
}

impl ApplicationHandler for WindowApp<'_> {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		/* TODO handle error? */
		let window = Arc::new(event_loop.create_window(Window::default_attributes()
			.with_title("Patina")
			.with_inner_size(LogicalSize::new(276, 256))).unwrap());

		/* TODO handle error? */
		self.pixels = {
			let window_size = window.inner_size();
			let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window.clone());
			Some(Pixels::new(256, 240, surface_texture).unwrap())
		};
		self.window = Some(window);
		self.window.as_mut().unwrap().request_redraw();
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _ : WindowId, event: WindowEvent) {
		match event {
			WindowEvent::RedrawRequested => {
				let pixels = self.pixels.as_mut().unwrap();
				let frame = pixels.frame_mut();

				frame.copy_from_slice(self.write_buffer.lock().unwrap().deref());

				let _ = pixels.render();
				self.window.as_mut().unwrap().request_redraw();
			}
			WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			WindowEvent::Resized(size) => {
				self.pixels.as_mut().unwrap().resize_surface(size.width, size.height).expect("TODO: panic message");
			}
			WindowEvent::KeyboardInput { device_id: _, event: input, is_synthetic: _ } => {
				match input.state {
					ElementState::Pressed => {
						self.keys.lock().unwrap().insert(input.logical_key);
					},
					ElementState::Released => {
						self.keys.lock().unwrap().remove(&input.logical_key);
					}
				}
			}
			_ => ()
		}
	}
}

pub fn initialize_ui(write_buffer : Arc<Mutex<WriteBuffer>>, keys : Arc<Mutex<HashSet<Key>>>) -> Result<(), EventLoopError> {
	let event_loop = EventLoop::new();
	event_loop?.run_app(&mut WindowApp::new(write_buffer, keys))

	// let quantum = Duration::from_millis(10);
}