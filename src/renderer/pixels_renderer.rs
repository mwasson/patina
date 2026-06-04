//! Windows/macOS renderer: the native menu lives outside the window's client
//! area, so pixels' full-window wgpu surface is fine and gives free GPU
//! scaling. This file is not compiled on Linux.

use crate::ppu::{WriteBuffer, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use pixels::{Pixels, SurfaceTexture};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tao::window::Window;

pub(crate) struct PixelsRenderer {
    pixels: Pixels<'static>,
    write_buffer: Arc<Mutex<WriteBuffer>>,
}

impl PixelsRenderer {
    pub(crate) fn new(window: &Arc<Window>, write_buffer: Arc<Mutex<WriteBuffer>>) -> Self {
        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());
        let pixels = Pixels::new(DISPLAY_WIDTH, DISPLAY_HEIGHT, surface_texture)
            .expect("Failed to create pixels surface");
        Self {
            pixels,
            write_buffer,
        }
    }

    pub(crate) fn render(&mut self) {
        let frame = self.pixels.frame_mut();
        frame.copy_from_slice(self.write_buffer.lock().unwrap().deref());
        let _ = self.pixels.render();
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let _ = self.pixels.resize_surface(width, height);
    }

    pub(crate) fn set_write_buffer(&mut self, write_buffer: Arc<Mutex<WriteBuffer>>) {
        self.write_buffer = write_buffer;
    }
}
