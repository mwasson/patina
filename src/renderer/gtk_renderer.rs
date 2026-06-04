//! Linux/BSD renderer: paints the NES framebuffer into a `gtk::DrawingArea`
//! packed below muda's menubar, so GTK composites menu and frame correctly.

use crate::ppu::{WriteBuffer, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use crate::renderer::{fit_rect, rgba_to_cairo_rgb24};
use gtk::cairo::{Context, Filter, Format, ImageSurface};
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tao::platform::unix::WindowExtUnix;
use tao::window::Window;

/// The current framebuffer the draw callback reads from. Held behind
/// `Rc<RefCell<..>>` so loading a new ROM can swap in its buffer without
/// rebuilding the widget. GTK is single-threaded, so `Rc`/`RefCell` is sound.
type SharedBuffer = Rc<RefCell<Arc<Mutex<WriteBuffer>>>>;

pub(crate) struct GtkRenderer {
    drawing_area: gtk::DrawingArea,
    current_buffer: SharedBuffer,
}

impl GtkRenderer {
    pub(crate) fn new(window: &Arc<Window>, write_buffer: Arc<Mutex<WriteBuffer>>) -> Self {
        let drawing_area = gtk::DrawingArea::new();
        let current_buffer: SharedBuffer = Rc::new(RefCell::new(write_buffer));

        let draw_buffer = current_buffer.clone();
        drawing_area.connect_draw(move |area, cr| {
            let buffer_cell = draw_buffer.borrow();
            let buffer = buffer_cell.lock().unwrap();
            paint(area, cr, &buffer);
            glib::Propagation::Proceed
        });

        // muda forces its menubar to child position 0 of the vbox, so packing
        // here always lands the frame below it.
        if let Some(vbox) = window.default_vbox() {
            vbox.pack_start(&drawing_area, true, true, 0);
            vbox.show_all();
        }

        // Drive continuous redraws off GTK's frame clock. tao's Linux event loop
        // only wakes on GTK events and installs no timer for `WaitUntil`, so we
        // cannot rely on `MainEventsCleared` for periodic rendering; the frame
        // clock ticks at the display refresh rate and wakes the loop each tick.
        let _ = drawing_area.add_tick_callback(|area, _clock| {
            area.queue_draw();
            glib::ControlFlow::Continue
        });

        Self {
            drawing_area,
            current_buffer,
        }
    }

    pub(crate) fn render(&mut self) {
        // Redraws are driven by the frame-clock tick callback set up in `new`;
        // this also queues one in case the loop is woken by a window event.
        self.drawing_area.queue_draw();
    }

    pub(crate) fn resize(&mut self, _width: u32, _height: u32) {
        // GTK lays out the drawing area within the window automatically.
    }

    pub(crate) fn set_write_buffer(&mut self, write_buffer: Arc<Mutex<WriteBuffer>>) {
        *self.current_buffer.borrow_mut() = write_buffer;
    }
}

/// Paints `buffer` into the drawing area, scaled to fill while preserving
/// aspect ratio, with nearest-neighbor filtering for crisp pixels.
fn paint(area: &gtk::DrawingArea, cr: &Context, buffer: &WriteBuffer) {
    // Clear the whole widget to black first, so letterbox bars around the
    // aspect-fit frame show black rather than the window's default background.
    cr.set_source_rgb(0.0, 0.0, 0.0);
    let _ = cr.paint();

    let width = DISPLAY_WIDTH as usize;
    let height = DISPLAY_HEIGHT as usize;

    let Ok(stride) = Format::Rgb24.stride_for_width(DISPLAY_WIDTH) else {
        return;
    };

    let mut data = vec![0u8; stride as usize * height];
    rgba_to_cairo_rgb24(buffer, &mut data, width, height, stride as usize);

    let Ok(surface) = ImageSurface::create_for_data(
        data,
        Format::Rgb24,
        DISPLAY_WIDTH as i32,
        DISPLAY_HEIGHT as i32,
        stride,
    ) else {
        return;
    };

    let (x, y, w, h) = fit_rect(
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        area.allocated_width().max(0) as u32,
        area.allocated_height().max(0) as u32,
    );
    if w == 0 || h == 0 {
        return;
    }

    cr.translate(x as f64, y as f64);
    cr.scale(w as f64 / DISPLAY_WIDTH as f64, h as f64 / DISPLAY_HEIGHT as f64);
    if cr.set_source_surface(&surface, 0.0, 0.0).is_ok() {
        cr.source().set_filter(Filter::Nearest);
        let _ = cr.paint();
    }
}
