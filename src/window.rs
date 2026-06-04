use crate::key_event_handler::KeyEventHandler;
use crate::menu::{self, MenuAction};
use crate::renderer::Renderer;
use crate::rom::Rom;
use crate::simulator::program_state::ProgramState;
use muda::{Menu, MenuEvent, MenuId};
use std::error::Error;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::dpi::LogicalSize;
use tao::event::{ElementState, Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::keyboard::ModifiersState;
use tao::window::{Window, WindowBuilder};

const WINDOW_START_WIDTH: u16 = 420;
const WINDOW_START_HEIGHT: u16 = 380;
/// Target redraw cadence (~60 fps). The emulator runs on its own thread; the UI
/// just samples its framebuffer at this rate.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Events delivered to the event loop from sources other than window events:
/// the OS signal handler (Ctrl+C) and `muda` menu activations.
pub(crate) enum AppEvent {
    SaveAndExit,
    Menu(MenuId),
}

struct WindowApp {
    renderer: Renderer,
    key_event_handler: KeyEventHandler,
    program_state: ProgramState,
    savefile: Option<String>,
    modifiers: ModifiersState,
    /// The native menu bar. Kept alive for the lifetime of the app: dropping it
    /// removes the menu from the window.
    _menu: Menu,
}

impl WindowApp {
    fn render(&mut self) {
        self.renderer.render();
    }

    /// Routes every user-triggered action (menu item, keyboard shortcut, window
    /// close, or signal) through a single place.
    fn handle_action(&mut self, action: MenuAction, control_flow: &mut ControlFlow) {
        match action {
            MenuAction::LoadRom => self.load_rom(),
            MenuAction::Exit => self.do_exit(control_flow),
        }
    }

    fn do_exit(&mut self, control_flow: &mut ControlFlow) {
        let save_data = self.program_state.cleanup();
        if let (Some(path), Some(data)) = (&self.savefile, save_data) {
            if let Err(e) = fs::write(path, data) {
                eprintln!("Failed to write save file {path}: {e}");
            }
        }
        *control_flow = ControlFlow::Exit;
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
        self.renderer.set_write_buffer(new_state.write_buffer.clone());
        self.key_event_handler
            .set_write_buffer(new_state.write_buffer.clone());
        self.program_state = new_state;
    }

    fn window_event(&mut self, event: WindowEvent, control_flow: &mut ControlFlow) {
        match event {
            WindowEvent::CloseRequested => {
                self.do_exit(control_flow);
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers;
            }
            WindowEvent::KeyboardInput { event: input, .. } => {
                if input.state == ElementState::Pressed {
                    if let Some(action) =
                        menu::action_for_shortcut(self.modifiers.control_key(), &input.logical_key)
                    {
                        self.handle_action(action, control_flow);
                        return;
                    }
                }
                self.key_event_handler.handle_key_event(&input);
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event: AppEvent, control_flow: &mut ControlFlow) {
        match event {
            AppEvent::SaveAndExit => self.do_exit(control_flow),
            AppEvent::Menu(id) => {
                if let Some(action) = menu::action_for_menu_id(&id) {
                    self.handle_action(action, control_flow);
                }
            }
        }
    }
}

/// Attaches the menu bar to the window. This is the only platform-divergent
/// part of the menu implementation; the menu itself is defined once in
/// [`crate::menu`].
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn attach_menu(menu: &Menu, window: &Window) {
    use tao::platform::unix::WindowExtUnix;
    if let Err(e) = menu.init_for_gtk_window(window.gtk_window(), window.default_vbox()) {
        eprintln!("Failed to attach menu: {e}");
    }
}

#[cfg(target_os = "windows")]
fn attach_menu(menu: &Menu, window: &Window) {
    use tao::platform::windows::WindowExtWindows;
    if let Err(e) = unsafe { menu.init_for_hwnd(window.hwnd() as isize) } {
        eprintln!("Failed to attach menu: {e}");
    }
}

#[cfg(target_os = "macos")]
fn attach_menu(menu: &Menu, _window: &Window) {
    menu.init_for_nsapp();
}

pub fn initialize_ui(
    program_state: ProgramState,
    key_event_handler: KeyEventHandler,
    savefile: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();

    // Forward native menu activations into the event loop as user events.
    let menu_proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = menu_proxy.send_event(AppEvent::Menu(event.id().clone()));
    }));

    // Route Ctrl+C through the event loop so it shares the save-and-exit path.
    let signal_proxy = event_loop.create_proxy();
    ctrlc::set_handler(move || {
        let _ = signal_proxy.send_event(AppEvent::SaveAndExit);
    })
    .expect("Should not error due to being only signal handler");

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Patina")
            .with_inner_size(LogicalSize::new(WINDOW_START_WIDTH, WINDOW_START_HEIGHT))
            .build(&event_loop)?,
    );

    // Attach the menu before creating the renderer: on Linux the renderer packs
    // its drawing area into the same vbox, below muda's menubar.
    let menu = menu::build_menu()?;
    attach_menu(&menu, &window);

    let renderer = Renderer::new(&window, program_state.write_buffer.clone());

    let mut app = WindowApp {
        renderer,
        key_event_handler,
        program_state,
        savefile,
        modifiers: ModifiersState::empty(),
        _menu: menu,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + FRAME_INTERVAL);
        match event {
            Event::WindowEvent { event, .. } => app.window_event(event, control_flow),
            Event::UserEvent(app_event) => app.user_event(app_event, control_flow),
            Event::MainEventsCleared => app.render(),
            _ => (),
        }
    })
}
