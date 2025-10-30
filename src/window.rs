use crate::key_event_handler::KeyEventHandler;
use crate::ppu;
use crate::ppu::WriteBuffer;
use crate::scheduler::RenderRequester;
use pixels::{Pixels, SurfaceTexture};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

const WINDOW_START_WIDTH: u16 = 420;
const WINDOW_START_HEIGHT: u16 = 380;

struct WindowApp<'a> {
    write_buffer: Arc<Mutex<WriteBuffer>>,
    pixels: Option<Pixels<'a>>,
    window: Option<Arc<Window>>,
    requester: Arc<Mutex<RenderRequester>>,
    key_event_handler: KeyEventHandler,
}

impl WindowApp<'_> {
    fn new(
        write_buffer: Arc<Mutex<WriteBuffer>>,
        key_event_handler: KeyEventHandler,
        requester: Arc<Mutex<RenderRequester>>,
    ) -> Self {
        Self {
            write_buffer,
            requester,
            pixels: None,
            window: None,
            key_event_handler,
        }
    }
}

impl ApplicationHandler for WindowApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        /* TODO handle error? */
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Patina")
                        .with_inner_size(LogicalSize::new(WINDOW_START_WIDTH, WINDOW_START_HEIGHT)),
                )
                .unwrap(),
        );
        self.requester.lock().unwrap().set_window(window.clone());

        /* TODO handle error? */
        self.pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.clone());
            Some(Pixels::new(ppu::DISPLAY_WIDTH, ppu::DISPLAY_HEIGHT, surface_texture).unwrap())
        };

        self.window = Some(window);
        event_loop.set_control_flow(ControlFlow::Wait);
        self.window.as_mut().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                let pixels = self.pixels.as_mut().unwrap();
                let frame = pixels.frame_mut();

                frame.copy_from_slice(self.write_buffer.lock().unwrap().deref());

                let _ = pixels.render();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.pixels
                    .as_mut()
                    .unwrap()
                    .resize_surface(size.width, size.height)
                    .expect("TODO: panic message");
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event: input,
                is_synthetic: _,
            } => {
                self.key_event_handler.handle_key_event(&input);
            }
            _ => (),
        }
    }
}

pub fn initialize_ui(
    write_buffer: Arc<Mutex<WriteBuffer>>,
    key_event_handler: KeyEventHandler,
    requester: Arc<Mutex<RenderRequester>>,
) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new();
    event_loop?.run_app(&mut WindowApp::new(
        write_buffer,
        key_event_handler,
        requester,
    ))
}
