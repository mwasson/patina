//! Frame presentation.
//!
//! The renderer is platform-split because native menus are (see `Cargo.toml`):
//! [`PixelsRenderer`](pixels_renderer::PixelsRenderer) on Windows/macOS, where
//! the native menu is outside the client area and pixels' full-window wgpu
//! surface is fine; [`GtkRenderer`](gtk_renderer::GtkRenderer) on Linux/BSD,
//! where the GTK menubar is an in-window widget so we paint the frame into a
//! `gtk::DrawingArea` below it. Both expose the same
//! `new` / `render` / `resize` / `set_write_buffer` surface, so `window.rs`
//! stays platform-agnostic.

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod pixels_renderer;
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub(crate) use pixels_renderer::PixelsRenderer as Renderer;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod gtk_renderer;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) use gtk_renderer::GtkRenderer as Renderer;

// The helpers below are pure and used only by the GTK backend (pixels does its
// scaling and format conversion on the GPU). They live here, separate from the
// GTK glue, so they can be unit-tested without a display.
#[cfg(all(test, not(any(target_os = "windows", target_os = "macos"))))]
mod tests;

/// Computes the largest `src`-aspect-ratio rectangle that fits inside
/// `dst`, centered. Returns `(x, y, width, height)` in destination pixels.
/// Used to letterbox the NES frame into the drawing area. Pure.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn fit_rect(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (i32, i32, u32, u32) {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return (0, 0, 0, 0);
    }
    let scale = f64::min(dst_w as f64 / src_w as f64, dst_h as f64 / src_h as f64);
    let w = ((src_w as f64 * scale).round() as u32).max(1);
    let h = ((src_h as f64 * scale).round() as u32).max(1);
    let x = (dst_w as i32 - w as i32) / 2;
    let y = (dst_h as i32 - h as i32) / 2;
    (x, y, w, h)
}

/// Repacks an RGBA8 framebuffer into Cairo's `Rgb24` layout (little-endian
/// `0x00RRGGBB`, i.e. bytes `B, G, R, _`), honoring Cairo's row `stride`. Pure.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) fn rgba_to_cairo_rgb24(
    rgba: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
) {
    for y in 0..height {
        for x in 0..width {
            let s = (y * width + x) * 4;
            let d = y * stride + x * 4;
            dst[d] = rgba[s + 2]; // B
            dst[d + 1] = rgba[s + 1]; // G
            dst[d + 2] = rgba[s]; // R
            dst[d + 3] = 0; // unused
        }
    }
}
