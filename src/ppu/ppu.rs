use crate::cpu::CoreMemory;
use crate::mapper::Mapper;
use crate::ppu::palette::Palette;
use crate::ppu::{
    PPUInternalRegisters, Tile, WriteBuffer, OAM, OAM_SIZE, OVERSCAN, PALETTE_MEMORY_SIZE,
    VRAM_SIZE, WRITE_BUFFER_SIZE,
};
use crate::processor::Processor;
use std::cell::RefCell;
use std::cmp::min;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct PPU {
    memory: Rc<RefCell<CoreMemory>>,
    mapper: Rc<RefCell<Box<dyn Mapper>>>,
    pub(super) oam: OAM,
    write_buffer: Arc<Mutex<WriteBuffer>>,
    internal_buffer: WriteBuffer,
    vram: [u8; VRAM_SIZE],
    palette_memory: [u8; PALETTE_MEMORY_SIZE],
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
        memory: Rc<RefCell<CoreMemory>>,
    ) -> Rc<RefCell<PPU>> {
        let mapper = memory.clone().borrow().mapper.clone();
        Rc::new(RefCell::new(PPU {
            memory,
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

        self.internal_regs.copy_y_bits();
    }

    pub fn end_of_screen_render(&mut self) {
        /* set vblank flag */
        self.ppu_status = set_bit_on(self.ppu_status, 7);
        /* vblank NMI */
        if self.ppu_ctrl & (1 << 7) != 0 {
            self.memory.borrow_mut().set_nmi(true);
        }

        /* write new pixels so UI can see them */
        self.write_buffer
            .lock()
            .unwrap()
            .copy_from_slice(&self.internal_buffer);
    }

    pub fn render_scanline(&mut self, scanline: u8) {
        let scanline_sprites = self.sprite_evaluation(scanline);

        let mut line_buffer = [0; 256 * 4];

        /* technically should occur at end of previous scanline, but if the entire scanline occurs
         * at once, this guarantees the CPU has updated its side of things
         */
        self.internal_regs.copy_x_bits();

        let render_background = self.ppu_mask & (1 << 3) != 0;
        let render_sprites = self.ppu_mask & (1 << 4) != 0;

        if render_sprites {
            self.render_sprites(&scanline_sprites, scanline, &mut line_buffer, true);
        }
        if render_background {
            self.render_background_tiles(scanline, &mut line_buffer);
        }
        if render_sprites {
            self.render_sprites(&scanline_sprites, scanline, &mut line_buffer, false);
        }

        /* solid background color everywhere we didn't render a sprite or background tile */
        self.render_solid_background_color(&mut line_buffer);

        /* update scrolling */
        self.internal_regs.y_increment();

        if scanline >= OVERSCAN && scanline <= 240 - OVERSCAN {
            self.internal_buffer[Self::pixel_range_for_line(scanline)]
                .copy_from_slice(&line_buffer);
        }
    }

    fn render_background_tiles(&mut self, scanline: u8, line_buffer: &mut [u8; 1024]) {
        let sprite0 = self.slice_as_sprite(0);
        let mut sprite_zero_in_scanline_not_yet_found =
            self.ppu_status & (1 << 6) == 0 && sprite0.in_scanline(scanline, self.sprite_height());

        for x in (0..0x101).step_by(8) {
            let tile = self
                .get_bg_tile(self.read_vram((0x2000 | (self.internal_regs.v & 0xfff)) as usize));
            let palette = self.palette_for_current_bg_tile();
            let tile_offset = x as i16 - self.internal_regs.get_fine_x() as i16;
            for pixel_offset in 0..8 {
                let pixel_loc = tile_offset + pixel_offset as i16;
                if pixel_loc < 0 {
                    continue;
                }
                let index = pixel_loc as usize * 4;
                if pixel_loc > 0xff || line_buffer[index + 3] != 0 {
                    continue;
                }
                let brightness = tile.pixel_intensity(
                    &self.mapper.borrow(),
                    pixel_offset,
                    self.internal_regs.get_fine_y(),
                );

                if brightness > 0 {
                    /* TODO doesn't handle 16 pixel tall sprites */
                    line_buffer[index..(index + 4)]
                        .copy_from_slice(&palette.brightness_to_pixels(brightness));

                    /* sprite zero hit detection */
                    if sprite_zero_in_scanline_not_yet_found
                        && pixel_loc as u8 >= sprite0.x
                        && (pixel_loc as u8) < sprite0.x + 8
                        && sprite0.get_brightness_localized(
                            self,
                            pixel_loc as u8 - sprite0.x,
                            scanline - sprite0.get_y(),
                        ) > 0
                    {
                        sprite_zero_in_scanline_not_yet_found = false;
                        self.ppu_status = set_bit_on(self.ppu_status, 6);
                    }
                }
            }

            self.internal_regs.coarse_x_increment();
        }
    }

    fn render_solid_background_color(&mut self, line_buffer: &mut [u8; 1024]) {
        /* background color */
        let bg_pixels = self.get_palette(0).brightness_to_pixels(0);
        for x in 0..0x100 {
            if line_buffer[x * 4 + 3] == 0 {
                line_buffer[(x * 4)..(x * 4 + 4)].copy_from_slice(&bg_pixels);
            }
        }
    }

    fn render_sprites(
        &self,
        scanline_sprites: &Vec<SpriteInfo>,
        scanline: u8,
        line_buffer: &mut [u8],
        is_foreground: bool,
    ) {
        for sprite in scanline_sprites {
            if sprite.is_foreground() != is_foreground {
                continue;
            }

            let sprite_palette = sprite.get_palette(&self);
            for i in 0..min(8, (0xff - sprite.x).saturating_add(1)) {
                let brightness =
                    sprite.get_brightness_localized(self, i, scanline - sprite.get_y());
                /* TODO: bug here where sprite can wrap around the screen */
                let pixel_index = sprite.x.wrapping_add(i) as usize * 4;
                let pixels = sprite_palette.brightness_to_pixels(brightness);
                if brightness > 0 && line_buffer[pixel_index + 3] == 0 {
                    line_buffer[pixel_index..pixel_index + 4].copy_from_slice(&pixels);
                }
            }
        }
    }

    fn sprite_height(&self) -> u8 {
        if self.tall_sprites {
            16
        } else {
            8
        }
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

    /* Finds the first eight sprites on the given scanline, determined
     * by position in the OAM. Takes into account whether sprites are 8 or 16
     * pixels tall. It then copies these into secondary OAM. Also sets the
     * sprite overflow bit if necessary.
     */
    fn sprite_evaluation(&mut self, scanline_num: u8) -> Vec<SpriteInfo> {
        let mut scanline_sprites = Vec::new();
        for i in 0..OAM_SIZE / 4 {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self.sprite_height()) {
                /* already found eight sprites, set overflow */
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                if scanline_sprites.len() >= 8 {
                    self.ppu_status = set_bit_on(self.ppu_status, 1);
                    break;
                }
                scanline_sprites.push(sprite_data);
            }
        }
        scanline_sprites
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam[sprite_index * 4..sprite_index * 4 + 4]);
        SpriteInfo::from_memory(&sprite_data)
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
        self.mapper
            .borrow()
            .read_tile(tile_index, pattern_table_num)
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc: usize = (palette_index as usize) * 4;
        let mut palette_data = [0u8; 4];
        palette_data.copy_from_slice(&self.palette_memory[palette_mem_loc..palette_mem_loc + 4]);

        Palette::new(palette_data)
    }

    fn pixel_range_for_line(y: u8) -> core::ops::Range<usize> {
        let start = (y - OVERSCAN) as usize; /* don't show the first few lines */
        let range_width = 4 * 256; /* 4 bytes per pixel, 256 pixels per line */
        start * range_width..(start + 1) * range_width
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
            result = 0x3f00 | (result & 0xff);

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
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, sprite_height: u8) -> bool {
        self.get_y() <= scanline && scanline - self.get_y() < sprite_height
    }

    fn get_y(&self) -> u8 {
        self.y + 1
    }

    fn get_brightness_localized(&self, ppu: &PPU, x: u8, y: u8) -> u8 {
        let tile = ppu.get_sprite_tile(self.tile_index); /* TODO is this right? */
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
        tile.pixel_intensity(&ppu.mapper.borrow(), x_to_use, y_to_use)
    }

    fn get_palette(&self, ppu: &PPU) -> Palette {
        ppu.get_palette((self.attrs & 0x3) + 4)
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x20 == 0
    }

    /* create a SpriteInfo from memory */
    fn from_memory(src_slice: &[u8]) -> SpriteInfo {
        SpriteInfo {
            y: src_slice[0],
            tile_index: src_slice[1],
            attrs: src_slice[2],
            x: src_slice[3],
        }
    }
}

fn set_bit_on(flags: u8, bit: u8) -> u8 {
    flags | (1 << bit)
}

fn set_bit_off(flags: u8, bit: u8) -> u8 {
    flags & !(1 << bit)
}
