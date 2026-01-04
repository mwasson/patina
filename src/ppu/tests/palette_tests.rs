/* palette code doesn't fail if it's passed a hue with high order bits set */
use crate::ppu::palette::Palette;

#[test]
fn test_palette_high_order_colors() {
    assert_eq!(Palette::hue_lookup(0xff), Palette::hue_lookup(0xff & 0x3F));
}
