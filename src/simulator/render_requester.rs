use std::sync::Arc;
use winit::window::Window;

pub struct RenderRequester {
    window: Option<Arc<Window>>,
}

impl RenderRequester {
    pub fn new() -> RenderRequester {
        RenderRequester { window: None }
    }
    pub fn set_window(&mut self, window: Arc<Window>) {
        self.window = Some(window);
    }

    pub fn request_redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
