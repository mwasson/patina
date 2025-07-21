use crate::cpu::ProgramState;
use crate::rom::Rom;

use bincode;
use bincode::{BorrowDecode, Encode};
use crate::ppu::palette::Palette;
use crate::ppu::Tile;

const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
const OAM_SIZE : usize = 256;

const SECONDARY_OAM_SIZE : usize = 32;

/* TODO: ppumask rendering effects */

struct PPUState<'a> {
    vram: [u8; PPU_MEMORY_SIZE],
    oam: [u8; OAM_SIZE],
    secondary_oam: [u8; SECONDARY_OAM_SIZE],
    ppuctrl: &'a u8,
    ppustatus: u8, /* TODO: really need to figure out how I want to share this info! */
    oamaddr: &'a u8,
    oamdata: &'a u8,
    oamdma: &'a u8,
}

impl PPUState<'_> { /* TODO: how should the lifetime work here...? */
    pub fn from_rom<'a>(rom: &Rom, program_state: &'a ProgramState) -> PPUState<'a> {
        let mut vram: [u8; PPU_MEMORY_SIZE] = [0; PPU_MEMORY_SIZE];
        let oam : [u8; OAM_SIZE] = [0; OAM_SIZE]; /* TODO: link this to CPU memory? */

        /* copy over character data; TODO surely this is not correct even in the no-mapper case*/
        vram[0x0000..rom.chr_data.len()].copy_from_slice(&rom.chr_data);

        return PPUState {
            vram,
            oam,
            secondary_oam: [0; SECONDARY_OAM_SIZE],
            /* TODO: a simple link like this is not sufficient; how should it actually work? */
            ppuctrl: program_state.link_memory(0x2000),
            ppustatus: 0,// program_state.link_memory(0x2002),
            oamaddr: program_state.link_memory(0x2003),
            oamdata: program_state.link_memory(0x2004),
            oamdma: program_state.link_memory(0x4014),
        }
    }

    fn render_scanline(&self, scanline: u8, write_buffer: &mut [u8]) {
        let write_buffer_num_pixels = write_buffer.len()/4;
        for (pixel_buffer,i) in write_buffer.chunks_exact_mut(4).zip((0..write_buffer_num_pixels)) {
            // TODO remove just a placeholder
            let iu8 = i as u8;
            let sprite = SpriteInfo::from_memory(&self.secondary_oam[0..4]);
            let (fg_sprite, fg_brightness) = self.find_first_sprite(iu8, scanline, true);
            let (bg_pixels, bg_tile_brightness) = self.background_brightness(iu8, scanline);
            let (bg_sprite, bg_sprite_brightness) = self.find_first_sprite(iu8, scanline, false);

            let pixels = if fg_brightness > 0 {
                fg_sprite.unwrap().color_from_brightness(self, fg_brightness)
            } else if bg_tile_brightness > 0 {
                bg_pixels
            } else if bg_sprite_brightness > 0 {
                bg_sprite.unwrap().color_from_brightness(self, bg_sprite_brightness)
            } else {
                /* use the master background color, stored in color 0 of palette 0 */
                self.get_palette(0).brightness_to_pixels(0)
            };

            pixel_buffer.copy_from_slice(&pixels);
        }
    }

    fn find_first_sprite(&self, x: u8, y: u8, is_foreground: bool) -> (Option<SpriteInfo>, u8) {
        let mut brightness = 0;
        let mut sprite = Option::None;
        for sprite_data in self.secondary_oam.chunks_exact(4) {
            let cur_sprite = SpriteInfo::from_memory(sprite_data);
            if cur_sprite.at_x_position(x) && cur_sprite.is_foreground() == is_foreground {
                let sprite_brightness = cur_sprite.get_brightness(self, x, y);
                if(sprite_brightness != 0) {
                    brightness = sprite_brightness;
                    sprite = Some(cur_sprite);
                    break;
                }
            }
        }
        (sprite, brightness)
    }

    fn background_brightness(&self, x:u8, y:u8) -> ([u8;4], u8) {
        let tile = self.tile_for_pixel(x,y);
        let palette = self.palette_for_pixel(x,y);

        let tile_origin = (x - x % 8, y  - y%8);
        let brightness = tile.pixel_intensity((x - tile_origin.0) as usize, (y - tile_origin.1) as usize);

        (palette.brightness_to_pixels(brightness), brightness)
    }

    /* TODO; this only uses the first name table */
    fn tile_for_pixel(&self, x:u8, y:u8) -> Tile {
        let offset : usize = (y/8*32 + x/8) as usize;
        let tile_index = self.vram[0x2000 + offset];
        self.get_bg_tile(tile_index)
    }

    /* TODO: this only uses the first attribute table */
    fn palette_for_pixel(&self, x:u8, y:u8) -> Palette {
        /* each address controls a 32x32 pixel block; 8 blocks per row */
        let attr_addr = y/32*8 + x/32;
        let attr_table_value = self.vram[0x23c0 + attr_addr as usize];
        /* the attr_table_value stores information about 16x16 blocks as 2-bit palette references.
         * in order from the lowest bits they are: upper left, upper right, bottom left, bottom right
         */
        let attr_table_offset = if x % 32 < 16 && y % 32 < 16 {
            0
        } else if x % 32 >= 16 && y % 32 < 16 {
            2
        } else if x % 32 < 16 && y % 32 >= 16 {
            4
        } else {
            6
        };
        self.get_palette((attr_table_value >> attr_table_offset) & 3) /* only need two bits */
    }

    /* TODO: this is just a sketch; not really sure how I want to use 'cycle' yet */
    fn sprite_evaluation(&mut self, scanline_num:u8, cycle:u16) {
        /* first 64 cycles: clear secondary oam */
        if(cycle <= 64 && cycle % 2 == 0) {
            self.secondary_oam[(cycle/2) as usize] = 0xff;
        /* 65-256: sprite evaluation */
        } else if (cycle <= 256) {
            self.write_scanline_sprites(scanline_num);

        /* 257-320: sprite fetches */
        } else if (cycle <= 320) {

            /* TODO */
        /* 321-340 + 0 (?): background render pipeline initialization */
        } else {
            /* TODO */
        }
    }

    /* Finds the first eight sprites on the given scanline, determined
     * by position in the OAM. Takes into account whether sprites are 8 or 16
     * pixels tall. It then copies these into secondary OAM. Also sets the
     * sprite overflow bit if necessary.
     */
    fn write_scanline_sprites(&mut self, scanline_num: u8) {
        let mut i = 0;
        let sprite_size = 8; /* TODO */
        let mut sprites_found = 0;
        for i in (0..OAM_SIZE/4) {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(i as u8, &self) {
                /* already found eight sprites, set overflow */
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                if(sprites_found >= 8) {
                    self.ppustatus = self.ppustatus | 0x20;
                    break;
                }
                sprite_data.copy_to_mem(&mut self.secondary_oam[(sprites_found*4)..(sprites_found*4+4)]);
                sprites_found += 1;
            }
        }
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let oam_size = self.oam.len();
        if(sprite_index + 4 >= oam_size) {
            log::warn!("Sprite index out of range; wrapping but consider avoiding this");
        }

        SpriteInfo::from_memory(&self.oam[(sprite_index*4 % OAM_SIZE)..(sprite_index*4+4) % OAM_SIZE])
    }

    /* sprites are 8 pixels tall unless the 5th bit of PPUCTRL is true, then they're 16 */
    fn sprite_size(&self) -> u8 {
        if self.ppuctrl & 0x10 != 0 { 16 } else { 8 }
    }

    /* TODO: handle 8x16 sprites */
    fn get_sprite_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, (self.ppuctrl & 0x4).count_ones() as usize)
    }

    fn get_bg_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, (self.ppuctrl & 0x8).count_ones() as usize)
    }

    fn get_tile(&self, tile_index: u8, pattern_table_num: usize) -> Tile {
        let pattern_table_base : usize = 0x1000 * pattern_table_num;
        Tile::from_memory(&self.vram[pattern_table_base..(pattern_table_base+0x1000)].chunks_exact(16).nth(tile_index as usize).unwrap())
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc : usize = 0x3f00 + (palette_index as usize)*4;
        let palette_data = &self.vram[palette_mem_loc..palette_mem_loc+4];

        Palette::new(palette_data)
    }
}

#[derive(BorrowDecode,Encode)]
struct SpriteInfo
{
    y: u8,
    tile_index: u8,
    attrs: u8,
    x: u8
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, ppu: &PPUState) -> bool {
            self.y <= scanline &&  scanline < self.y + ppu.sprite_size()
    }

    fn at_x_position(&self, x: u8) -> bool {
        self.x <= x && self.x + 8 > x
    }

    fn get_brightness(&self, ppu: &PPUState, x:u8, y:u8) -> u8 {
        self.get_brightness_localized(ppu, x-self.x,y-self.y)
    }

    fn get_brightness_localized(&self, ppu: &PPUState, x:u8, y:u8) -> u8 {
        let tile = ppu.get_sprite_tile(self.tile_index); /* TODO is this right? */
        tile.pixel_intensity(x as usize, y as usize)
    }

    fn color_from_brightness(&self, ppu: &PPUState, brightness: u8) -> [u8; 4] {
        self.get_palette(&ppu).brightness_to_pixels(brightness)
    }

    fn get_palette<'a>(&self, ppu: &'a PPUState<'a>) -> Palette<'a> {
        ppu.get_palette(self.attrs & 0x3)
    }

    /* write this sprite as a byte array into memory */
    fn copy_to_mem(&self, dst_slice: &mut [u8]) {
        let result = bincode::encode_into_slice(self, dst_slice, bincode::config::standard());

        if result.is_err() {
            panic!("Failed to copy sprite; {0}", result.unwrap_err());
        }
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x10 == 0
    }

    /* create a SpriteInfo from memory
     * TODO: bincode creates a copy, using serde might allow just a view into memory
     */
    fn from_memory(src_slice: &[u8]) -> SpriteInfo {
        bincode::borrow_decode_from_slice(src_slice,
                                          bincode::config::standard()).unwrap()
            .0
    }
}