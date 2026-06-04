use crate::renderer::{fit_rect, rgba_to_cairo_rgb24};

#[test]
fn fit_rect_exact_fit_fills_destination() {
    assert_eq!(fit_rect(256, 240, 256, 240), (0, 0, 256, 240));
}

#[test]
fn fit_rect_integer_scale_fills_destination() {
    assert_eq!(fit_rect(256, 240, 512, 480), (0, 0, 512, 480));
}

#[test]
fn fit_rect_wider_destination_letterboxes_horizontally() {
    // 1000x480: limited by height (scale 2.0), so 512x480 centered with side bars.
    assert_eq!(fit_rect(256, 240, 1000, 480), (244, 0, 512, 480));
}

#[test]
fn fit_rect_taller_destination_letterboxes_vertically() {
    // 512x1000: limited by width (scale 2.0), so 512x480 centered with top/bottom bars.
    assert_eq!(fit_rect(256, 240, 512, 1000), (0, 260, 512, 480));
}

#[test]
fn fit_rect_zero_dimension_yields_empty() {
    assert_eq!(fit_rect(256, 240, 0, 480), (0, 0, 0, 0));
    assert_eq!(fit_rect(0, 240, 512, 480), (0, 0, 0, 0));
}

#[test]
fn rgba_to_cairo_rgb24_swaps_channels_to_bgrx() {
    // one pixel: R=10, G=20, B=30, A=40 -> B,G,R,unused
    let rgba = [10u8, 20, 30, 40];
    let mut dst = [0u8; 4];
    rgba_to_cairo_rgb24(&rgba, &mut dst, 1, 1, 4);
    assert_eq!(dst, [30, 20, 10, 0]);
}

#[test]
fn rgba_to_cairo_rgb24_honors_row_stride_padding() {
    // 2x1 image, destination stride 12 (4 bytes padding past the 8 used).
    let rgba = [1u8, 2, 3, 255, 4, 5, 6, 255];
    let mut dst = [0xAAu8; 12];
    rgba_to_cairo_rgb24(&rgba, &mut dst, 2, 1, 12);
    // pixel 0 -> B,G,R,_ ; pixel 1 -> B,G,R,_ ; bytes 8..12 untouched.
    assert_eq!(&dst[0..4], &[3, 2, 1, 0]);
    assert_eq!(&dst[4..8], &[6, 5, 4, 0]);
    assert_eq!(&dst[8..12], &[0xAA, 0xAA, 0xAA, 0xAA]);
}
