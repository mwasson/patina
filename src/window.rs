use crate::key_event_handler::KeyEventHandler;
use crate::ppu;
use crate::ppu::WriteBuffer;
use crate::rom::Rom;
use crate::simulator::program_state::ProgramState;
use pixels::{Pixels, SurfaceTexture};
use std::fs;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState};
use winit::window::{Window, WindowId};

const WINDOW_START_WIDTH: u16 = 420;
const WINDOW_START_HEIGHT: u16 = 380;

pub(crate) enum AppEvent {
    SaveAndExit,
}

struct WindowApp<'a> {
    write_buffer: Arc<Mutex<WriteBuffer>>,
    pixels: Option<Pixels<'a>>,
    window: Option<Arc<Window>>,
    key_event_handler: KeyEventHandler,
    program_state: ProgramState,
    savefile: Option<String>,
    modifiers: ModifiersState,
}

impl WindowApp<'_> {
    fn new(
        key_event_handler: KeyEventHandler,
        program_state: ProgramState,
        savefile: Option<String>,
    ) -> Self {
        let write_buffer = program_state.write_buffer.clone();
        Self {
            write_buffer,
            pixels: None,
            window: None,
            key_event_handler,
            program_state,
            savefile,
            modifiers: ModifiersState::empty(),
        }
    }

    fn render(&mut self) {
        if let Some(pixels) = self.pixels.as_mut() {
            let frame = pixels.frame_mut();
            frame.copy_from_slice(self.write_buffer.lock().unwrap().deref());
            let _ = pixels.render();
        }
    }

    fn do_exit(&mut self, event_loop: &ActiveEventLoop) {
        let save_data = self.program_state.cleanup();
        if let (Some(path), Some(data)) = (&self.savefile, save_data) {
            if let Err(e) = fs::write(path, data) {
                eprintln!("Failed to write save file {path}: {e}");
            }
        }
        event_loop.exit();
    }

    fn load_rom(&mut self) {
        let path = rfd::FileDialog::new()
            .add_filter("NES ROM", &["nes"])
            .pick_file();

        let Some(path) = path else { return };

        let rom = match Rom::parse_file(path.to_string_lossy().to_string()) {
            Ok(rom) => rom,
            Err(e) => {
                eprintln!("Failed to load ROM: {e}");
                return;
            }
        };

        let key_source = self.program_state.key_source.clone();
        self.program_state.cleanup();
        let new_state = ProgramState::simulate_async(&rom, &None, key_source);
        self.write_buffer = new_state.write_buffer.clone();
        self.key_event_handler.set_write_buffer(new_state.write_buffer.clone());
        self.program_state = new_state;
    }
}

impl ApplicationHandler<AppEvent> for WindowApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Patina")
                        .with_inner_size(LogicalSize::new(WINDOW_START_WIDTH, WINDOW_START_HEIGHT)),
                )
                .unwrap(),
        );

        self.pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.clone());
            Some(Pixels::new(ppu::DISPLAY_WIDTH, ppu::DISPLAY_HEIGHT, surface_texture).unwrap())
        };

        self.window = Some(window);
        event_loop.set_control_flow(ControlFlow::Wait);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CloseRequested => {
                self.do_exit(event_loop);
            }
            WindowEvent::Resized(size) => {
                self.pixels
                    .as_mut()
                    .unwrap()
                    .resize_surface(size.width, size.height)
                    .expect("TODO: panic message");
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            WindowEvent::KeyboardInput { event: input, .. } => {
                if input.state == ElementState::Pressed && self.modifiers.control_key() {
                    match &input.logical_key {
                        Key::Character(c) if c.as_str() == "q" => {
                            self.do_exit(event_loop);
                            return;
                        }
                        Key::Character(c) if c.as_str() == "o" => {
                            self.load_rom();
                            return;
                        }
                        _ => {}
                    }
                }
                self.key_event_handler.handle_key_event(&input);
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::SaveAndExit => self.do_exit(event_loop),
        }
    }
}

pub fn initialize_ui(
    program_state: ProgramState,
    key_event_handler: KeyEventHandler,
    savefile: Option<String>,
) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    ctrlc::set_handler(move || {
        let _ = proxy.send_event(AppEvent::SaveAndExit);
    })
    .expect("Should not error due to being only signal handler");

    event_loop.run_app(&mut WindowApp::new(key_event_handler, program_state, savefile))
}
