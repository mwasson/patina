use crate::cpu::CPU;
use crate::mapper::Mapper;
use crate::ppu::palette::Palette;
use crate::ppu::{
    PPUInternalRegisters, Tile, WriteBuffer, OAM, OAM_SIZE, OVERSCAN, PALETTE_MEMORY_SIZE,
    VRAM_SIZE, WRITE_BUFFER_SIZE,
};
use crate::processor::Processor;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct PPU {
    mapper: Rc<RefCell<Box<dyn Mapper>>>,
    pub(super) oam: OAM,
    write_buffer: Arc<Mutex<WriteBuffer>>,
    internal_buffer: WriteBuffer,
    vram: [u8; VRAM_SIZE],
    palette_memory: [u8; PALETTE_MEMORY_SIZE],
    scanline_sprites: Option<Vec<SpriteInfo>>,
    current_tile: Option<Tile>,
    current_palette: Option<Palette>,
    // tick_count: u16,
    /* shared registers */
    pub(super) ppu_ctrl: u8,
    pub(super) ppu_mask: u8,
    pub(super) ppu_status: u8,
    pub(super) tall_sprites: bool, /* if true, sprites are 16 pixels tall instead of 8 */
    pub(super) internal_regs: PPUInternalRegisters,
}

#[derive(Clone, Debug)]
pub enum NametableMirroring {
    Horizontal,       /* pages are mirrored horizontally (appropriate for vertical games) */
    Vertical,         /* pages are mirrored vertically (appropriate for horizontal games) */
    SingleNametable0, /* first nametable mirrored four times */
    SingleNametable1, /* second nametable mirrored four times */
    #[allow(dead_code)]
    FourScreen, /* all four nametable/attribute tables available */
}

impl Processor for PPU {
    fn clock_speed(&self) -> u64 {
        1790000 * 3 /* 3x as fast as the CPU */
    }
}

impl PPU {
    pub fn new(
        write_buffer: Arc<Mutex<WriteBuffer>>,
        mapper: Rc<RefCell<Box<dyn Mapper>>>,
    ) -> Rc<RefCell<PPU>> {
        Rc::new(RefCell::new(PPU {
            mapper,
            write_buffer,
            oam: [0; OAM_SIZE],
            vram: [0; VRAM_SIZE],
            palette_memory: [0; PALETTE_MEMORY_SIZE],
            internal_buffer: [0; WRITE_BUFFER_SIZE],
            ppu_status: 0,
            ppu_ctrl: 0,
            ppu_mask: 0,
            internal_regs: PPUInternalRegisters::default(),
            tall_sprites: false,
            scanline_sprites: None,
            current_tile: None,
            current_palette: None,
        }))
    }

    // pub fn tick(&mut self) {
    //     let scanline = self.tick_count / 340;
    //     let dot = self.tick_count % 340;
    //
    //     if scanline <= 239 {
    //         self.render_scanline(scanline, dot);
    //     } else if scanline == 240 && dot == 0 {
    //
    //     } else if scanline == 241 && dot == 1 {
    //         /* set vblank flag */
    //         self.ppu_status = set_bit_on(self.ppu_status, 7);
    //     } else if scanline == 261 {
    //
    //     }
    //
    //
    //     self.tick_count += 1;
    // }

    pub fn beginning_of_screen_render(&mut self) {
        /* dummy scanline */
        /* clear VBlank */
        self.ppu_status = set_bit_off(self.ppu_status, 7);
        /* clear sprite 0 hit flag */
        self.ppu_status = set_bit_off(self.ppu_status, 6);
        /* clear overflow flag */
        self.ppu_status = set_bit_off(self.ppu_status, 5);

        if self.ppu_mask & 0x18 != 0 {
            self.internal_regs.copy_y_bits();
        }
    }

    pub fn end_of_screen_render(&mut self, cpu: &mut CPU) {
        /* set vblank flag */
        self.ppu_status = set_bit_on(self.ppu_status, 7);
        /* vblank NMI */
        if self.ppu_ctrl & (1 << 7) != 0 {
            cpu.set_nmi(true);
        }

        /* write new pixels so UI can see them */
        self.write_buffer
            .lock()
            .unwrap()
            .copy_from_slice(&self.internal_buffer);
    }

    pub fn render_scanline_begin(&mut self, scanline: u8) {
        let sprite_data = self.sprite_evaluation(scanline);
        self.scanline_sprites = Some(sprite_data.0);
        self.current_tile = Some(self.get_current_tile());
        self.current_palette = Some(self.palette_for_current_bg_tile());
    }

    pub fn render_scanline_end(&mut self) {
        if self.ppu_mask & 0x18 != 0 {
            self.internal_regs.copy_x_bits();
            self.internal_regs.y_increment();
        }
    }

    pub fn render_pixel(&mut self, scanline: u8, x: u8) {
        if scanline < OVERSCAN || scanline > 240 - OVERSCAN {
            return;
        }

        let render_background = self.ppu_mask & (1 << 3) != 0;
        let render_sprites = self.ppu_mask & (1 << 4) != 0;

        let pixel = 'pixel: {
            let mut background_sprite_pixel = None;
            if render_sprites {
                if let Some(pixel_data) =
                    self.render_sprites(&self.scanline_sprites.as_ref().unwrap(), scanline, x)
                {
                    self.sprite0_hit_detection(scanline, x, &pixel_data.0);
                    if pixel_data.0.is_foreground() {
                        break 'pixel pixel_data.1;
                    } else {
                        background_sprite_pixel = Some(pixel_data.1)
                    }
                }
            }
            if render_background {
                if let Some(pixel) = self.render_background_tiles(x) {
                    break 'pixel pixel;
                }
            }
            if background_sprite_pixel.is_some() {
                break 'pixel background_sprite_pixel.unwrap();
            }
            /* if no sprites or bg tile, render the global background color */
            self.render_background_color()
        };
        let index = scanline as usize * 1024 + x as usize * 4;
        self.internal_buffer[index..index + 4].copy_from_slice(pixel);
        if x % 8 + self.internal_regs.get_fine_x() == 7 {
            let coarse_x = if render_sprites || render_background {
                self.internal_regs.coarse_x_increment()
            } else {
                self.internal_regs.get_coarse_x()
            };
            self.current_tile = Some(self.get_current_tile());
            if coarse_x % 2 == 0 {
                self.current_palette = Some(self.palette_for_current_bg_tile());
            }
        }
    }

    fn render_background_tiles(&mut self, x: u8) -> Option<&'static [u8; 4]> {
        let brightness = self.current_tile.as_mut().unwrap().pixel_intensity(
            x - (self.internal_regs.get_coarse_x() * 8 - self.internal_regs.get_fine_x()),
            self.internal_regs.get_fine_y(),
        );

        if brightness > 0 {
            Some(
                self.current_palette
                    .as_ref()
                    .unwrap()
                    .brightness_to_pixels(brightness),
            )
        } else {
            None
        }
    }

    fn render_sprites(
        &self,
        scanline_sprites: &Vec<SpriteInfo>,
        scanline: u8,
        x: u8,
    ) -> Option<(SpriteInfo, &'static [u8; 4])> {
        for sprite in scanline_sprites {
            if x < sprite.x || x > sprite.x + 7 {
                continue;
            }
            let brightness =
                sprite.get_brightness_localized(self, x - sprite.x, scanline - sprite.get_y());
            if brightness > 0 {
                return Some((
                    sprite.clone(),
                    sprite.get_palette(&self).brightness_to_pixels(brightness),
                ));
            }
        }
        None
    }

    fn sprite0_hit_detection(&mut self, scanline: u8, x: u8, sprite_rendered: &SpriteInfo) {
        /* nothing to do if sprite 0 wasn't hit, or sprite zero has already been found */
        if sprite_rendered.sprite_index != 0 || self.ppu_status & (1 << 6) != 0 {
            return;
        }
        /* check to see if we're already rendering a background pixel; if so, it's a hit */
        if self.current_tile.as_mut().unwrap().pixel_intensity(
            x - (self.internal_regs.get_coarse_x() * 8 - self.internal_regs.get_fine_x()),
            self.internal_regs.get_fine_y(),
        ) > 0
        {
            self.ppu_status |= 1 << 6;
        }
    }

    fn sprite_height(&self) -> u8 {
        if self.tall_sprites {
            16
        } else {
            8
        }
    }

    fn get_current_tile(&self) -> Tile {
        self.get_bg_tile(self.read_vram((0x2000 | (self.internal_regs.v & 0xfff)) as usize))
    }

    fn palette_for_current_bg_tile(&self) -> Palette {
        /* TODO comment */
        /* 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07) */
        let addr = 0x23c0
            | ((self.internal_regs.get_nametable() as u16) << 10)
            | (((self.internal_regs.get_coarse_y() as u16) & 0x1c) << 1)
            | ((self.internal_regs.get_coarse_x() as u16) >> 2);
        /* each address controls a 32x32 pixel block; 8 blocks per row */
        let attr_table_value = self.read_vram(addr as usize);
        /* the attr_table_value stores information about 16x16 blocks as 2-bit palette references.
         * in order from the lowest bits they are: upper left, upper right, bottom left, bottom right
         */
        let x_low = 8 * self.internal_regs.get_coarse_x() % 32 < 16;
        let y_low = 8 * self.internal_regs.get_coarse_y() % 32 < 16;
        let attr_table_offset = if x_low {
            if y_low {
                0
            } else {
                4
            }
        } else {
            if y_low {
                2
            } else {
                6
            }
        };
        self.get_palette((attr_table_value >> attr_table_offset) & 3) /* only need two bits */
    }


    /* optimized version of looking up the global background color that skips
     * constructing a palette
     */
    fn render_background_color(&self) -> &'static [u8;4] {
        Palette::hue_lookup(self.palette_memory[0] as usize)
    }

    /* Finds the first eight sprites on the given scanline, determined
     * by position in the OAM. Takes into account whether sprites are 8 or 16
     * pixels tall. It then copies these into secondary OAM. Also sets the
     * sprite overflow bit if necessary.
     */
    fn sprite_evaluation(&mut self, scanline_num: u8) -> (Vec<SpriteInfo>, Option<SpriteInfo>) {
        let mut scanline_sprites = Vec::new();
        let mut sprite0 = None;
        let found_sprite0 = self.ppu_status & (1 << 6) != 0;
        let mut sprites_found = 0;
        for i in 0..OAM_SIZE / 4 {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self.sprite_height()) {
                scanline_sprites.push(sprite_data);
                if i == 0 && !found_sprite0 {
                    sprite0 = Some(sprite_data);
                }
                sprites_found += 1;
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                /* already found eight sprites, set overflow */

                if sprites_found == 8 {
                    self.ppu_status = set_bit_on(self.ppu_status, 1);
                    break;
                }
            }
        }
        (scanline_sprites, sprite0)
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam[sprite_index * 4..sprite_index * 4 + 4]);
        SpriteInfo::from_memory(sprite_index, &sprite_data)
    }

    fn get_sprite_tile(&self, tile_index: u8) -> Tile {
        let (tile_index, pattern_table) = if self.tall_sprites {
            (tile_index & !1, tile_index & 1)
        } else {
            (tile_index, (self.ppu_ctrl & 0x8) >> 3)
        };
        self.get_tile(tile_index, pattern_table)
    }

    fn get_bg_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, (self.ppu_ctrl & 0x10) >> 4)
    }

    fn get_tile(&self, tile_index: u8, pattern_table_num: u8) -> Tile {
        let pattern_table_base = 0x1000u16 * pattern_table_num as u16;
        let tile_start = pattern_table_base + (tile_index as u16 * 16);
        Tile::new(tile_start, self.mapper.clone())
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc: usize = (palette_index as usize) * 4;
        let mut palette_data = [0u8; 4];
        palette_data.copy_from_slice(&self.palette_memory[palette_mem_loc..palette_mem_loc + 4]);

        Palette::new(palette_data)
    }

    pub fn read_vram(&self, addr: usize) -> u8 {
        let mapped_address = self.vram_address_mirror(addr);

        /* pattern tables (CHR data) */
        if mapped_address < 0x2000 {
            self.mapper.borrow().read_chr(mapped_address as u16)
        /* nametables and attribute tables */
        } else if mapped_address < 0x3000 {
            self.vram[mapped_address - 0x2000]
        /* palettes */
        } else {
            self.palette_memory[mapped_address - 0x3f00]
        }
    }

    pub fn write_vram(&mut self, addr: usize, val: u8) {
        let mapped_address = self.vram_address_mirror(addr);

        /* pattern tables (CHR data) */
        if mapped_address < 0x2000 {
            self.mapper
                .borrow_mut()
                .write_chr(mapped_address as u16, val);
        /* nametables and attribute tables */
        } else if mapped_address < 0x3f00 {
            self.vram[mapped_address - 0x2000] = val;
            /* palettes */
        } else {
            self.palette_memory[mapped_address - 0x3f00] = val;
        }
    }

    pub fn vram_address_mirror(&self, addr: usize) -> usize {
        let mut result = addr;

        if result < 0x2000 {
            result
        } else if result < 0x3f00 {
            if result >= 0x3000 {
                result -= 0x1000;
            }

            /* TODO document */
            match self.mapper.borrow().get_nametable_mirroring() {
                NametableMirroring::Horizontal => result & !0x0800,
                NametableMirroring::Vertical => (result & !0x0C00) | ((result & 0x0800) >> 1),
                NametableMirroring::SingleNametable0 => result & !0x0c00,
                NametableMirroring::SingleNametable1 => (result & !0x0c00) + 0x400,
                NametableMirroring::FourScreen => result,
            }
        } else {
            /* palettes are repeated above 0x3f1f */
            result = 0x3f00 | (result & 0x1f);

            /* the first color of corresponding background and sprite palettes are shared;
             * this doesn't have any real effect, except if the true background color is
             * written to 0x3f10
             */
            if result == 0x3f10 {
                result -= 0x10;
            }

            result
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SpriteInfo {
    /* NB: this is one less than the top of the sprite! you'll have to add 1 whenever you use it (see get_y) */
    y: u8,
    tile_index: u8,
    attrs: u8,
    x: u8,
    sprite_index: usize,
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, sprite_height: u8) -> bool {
        self.get_y() <= scanline && scanline - self.get_y() < sprite_height
    }

    fn get_y(&self) -> u8 {
        self.y + 1
    }

    fn get_brightness_localized(&self, ppu: &PPU, x: u8, y: u8) -> u8 {
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

    fn get_palette(&self, ppu: &PPU) -> Palette {
        ppu.get_palette((self.attrs & 0x3) + 4)
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x20 == 0
    }

    /* create a SpriteInfo from memory */
    fn from_memory(sprite_index: usize, src_slice: &[u8]) -> SpriteInfo {
        SpriteInfo {
            y: src_slice[0],
            tile_index: src_slice[1],
            attrs: src_slice[2],
            x: src_slice[3],
            sprite_index,
        }
    }
}

fn set_bit_on(flags: u8, bit: u8) -> u8 {
    flags | (1 << bit)
}

fn set_bit_off(flags: u8, bit: u8) -> u8 {
    flags & !(1 << bit)
}
