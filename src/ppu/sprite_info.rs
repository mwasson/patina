use crate::ppu::palette::Palette;
use crate::ppu::PPU;

#[derive(Debug, Clone)]
pub struct SpriteInfo {
    /* NB: this is one less than the top of the sprite! you'll have to add 1 whenever you use it (see get_y) */
    y: u8,
    tile_index: u8,
    attrs: u8,
    pub(super) x: u8,
    pub(super) sprite_index: usize,
}

impl SpriteInfo {
    pub(super) fn in_scanline(&self, scanline: u8, sprite_height: u8) -> bool {
        let y = self.get_y();
        y <= scanline && scanline - y < sprite_height
    }

    pub(super) fn get_y(&self) -> u8 {
        self.y.saturating_add(1)
    }

    pub(super) fn get_brightness_localized(&self, ppu: &PPU, x: u8, y: u8) -> u8 {
        let mut tile = ppu.get_sprite_tile(self.tile_index);
        let mut x_to_use = x;
        if self.attrs & 0x40 != 0 {
            /* flipped horizontally */
            x_to_use = 7 - x_to_use;
        }
        let mut y_to_use = y;
        if self.attrs & 0x80 != 0 {
            /* flipped vertically */
            y_to_use = (ppu.sprite_height() - 1) - y_to_use;
        }
        tile.pixel_intensity(x_to_use, y_to_use)
    }

    pub(super) fn get_palette(&self, ppu: &PPU) -> Palette {
        ppu.get_palette((self.attrs & 0x3) + 4)
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x20 == 0
    }

    /* create a SpriteInfo from memory */
    pub(super) fn from_memory(sprite_index: usize, src_slice: &[u8]) -> SpriteInfo {
        SpriteInfo {
            y: src_slice[0],
            tile_index: src_slice[1],
            attrs: src_slice[2],
            x: src_slice[3],
            sprite_index,
        }
    }
}
