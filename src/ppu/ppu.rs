use crate::cpu::CPU;
use crate::mapper::Mapper;
use crate::ppu::palette::Palette;
use crate::ppu::sprite_info::SpriteInfo;
use crate::ppu::{
    PPUInternalRegisters, Tile, WriteBuffer, OAM, OAM_SIZE, OVERSCAN, PALETTE_MEMORY_SIZE,
    VRAM_SIZE, WRITE_BUFFER_SIZE,
};
use crate::processor::Processor;
use std::cell::RefCell;
use std::mem::replace;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use crate::scheduler::RenderRequester;

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
    next_tile: Option<Tile>,
    next_palette: Option<Palette>,
    tick_count: u32,
    is_even_frame: bool,
    /* shared registers */
    pub(super) ppu_ctrl: u8,
    pub(super) ppu_mask: u8,
    pub(super) oam_addr: u8,
    pub(super) ppu_status: u8,
    pub(super) tall_sprites: bool, /* if true, sprites are 16 pixels tall instead of 8 */
    pub(super) internal_regs: PPUInternalRegisters,
    requester: Arc<Mutex<RenderRequester>>,
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
        requester: Arc<Mutex<RenderRequester>>,
    ) -> Rc<RefCell<PPU>> {
        Rc::new(RefCell::new(PPU {
            mapper,
            write_buffer,
            oam: [0; OAM_SIZE],
            vram: [0; VRAM_SIZE],
            palette_memory: [0; PALETTE_MEMORY_SIZE],
            internal_buffer: [0; WRITE_BUFFER_SIZE],
            tick_count: 0,
            ppu_status: 0,
            ppu_ctrl: 0,
            ppu_mask: 0,
            oam_addr: 0,
            internal_regs: PPUInternalRegisters::default(),
            tall_sprites: false,
            scanline_sprites: None,
            current_tile: None,
            current_palette: None,
            next_tile: None,
            next_palette: None,
            is_even_frame: false,
            requester,
        }))
    }

    pub fn tick(&mut self, cpu: &mut CPU) {
        /* skip (0,0) on even frames */
        if(self.tick_count == 0 && self.is_even_frame) {
            self.tick_count += 1;
        }

        let scanline = self.tick_count / 341;
        let dot = (self.tick_count % 341) as u16;
        let rendering_on = self.ppu_mask & 0x18 != 0;

        if scanline < 240 {
            self.render_scanline(scanline as u8, dot, rendering_on);
        } else if scanline == 240 {
            if dot == 1 {
                // todo!()
            }
        } else if scanline == 241 && dot == 1 {
            self.end_of_screen_render(cpu);
        } else if scanline == 261 {
            self.prerender_scanline(dot, rendering_on);
        }

        if self.tick_count == 341*262 - 1 {
            self.tick_count = 0;
            self.is_even_frame = !self.is_even_frame;
        } else {
            self.tick_count += 1;
        }
    }

    fn render_scanline(&mut self, scanline: u8, dot: u16, rendering_on: bool) {
        if dot == 0 {
            self.render_scanline_begin(scanline);
        } else if dot < 257 {
            self.render_block(scanline, (dot - 1) as u8, rendering_on);
            if dot == 256 && rendering_on {
                self.internal_regs.y_increment();
            }
        } else if dot == 257 && rendering_on {
            self.internal_regs.copy_x_bits();
        } else if dot > 320 && dot < 329 {//337 TODO fix {
            self.render_block( (scanline + 1) % 240, (dot - 1 - 320) as u8, rendering_on);
        }
    }

    fn prerender_scanline(&mut self, dot: u16, rendering_on: bool) {
        if dot == 1 {
            /* clear overflow flag */
            set_bit_off(&mut self.ppu_status, 5);
            /* clear sprite 0 hit flag */
            set_bit_off(&mut self.ppu_status, 6);
            /* clear vblank flag */
            set_bit_off(&mut self.ppu_status, 7);
            /* TODO also ends PPU init period on startup/reset */
        } else if dot > 279 && dot < 305 {
            if rendering_on {
                self.internal_regs.copy_y_bits();
            }
        }
        self.render_scanline(0xff, dot, rendering_on);
    }

    fn render_block(&mut self, scanline: u8, x: u8, rendering_on: bool) {
        let mod8 = x % 8;
        /* NB: these are offset by 1 from actual dot number */
        if mod8 == 0 {
            self.load_tile();
            self.load_palette();
        }
        if rendering_on {
            self.render_pixel(scanline, x);

            if mod8 == 7 {
                self.internal_regs.coarse_x_increment();
            }
        }
    }

    pub fn end_of_screen_render(&mut self, cpu: &mut CPU) {
        /* set vblank flag */
        set_bit_on(&mut self.ppu_status, 7);
        /* vblank NMI */
        if self.ppu_ctrl & (1 << 7) != 0 {
            cpu.set_nmi(true);
        }

        /* write new pixels so UI can see them */
        self.write_buffer
            .lock()
            .unwrap()
            .copy_from_slice(&self.internal_buffer);

        self.requester.lock().unwrap().request_redraw();
    }

    pub fn render_scanline_begin(&mut self, scanline: u8) {
        let sprite_data = self.sprite_evaluation(scanline);
        self.scanline_sprites = Some(sprite_data);
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
        let render_background =
            self.ppu_mask & (1 << 3) != 0 && (x > 7 || self.ppu_mask & (1 << 1) != 0);
        let render_sprites =
            self.ppu_mask & (1 << 4) != 0 && (x > 7 || self.ppu_mask & (1 << 2) != 0);

        let pixel = 'pixel: {
            let mut background_sprite_pixel = None;
            if render_sprites {
                if let Some(pixel_data) =
                    self.render_sprites(&self.scanline_sprites.as_ref().unwrap(), scanline, x)
                {
                    if render_background {
                        self.sprite0_hit_detection(x, &pixel_data.0);
                    }
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

        if (scanline >= OVERSCAN) && (scanline <= 240 - OVERSCAN) {
            let index = scanline as usize * 1024 + x as usize * 4;
            self.internal_buffer[index..index + 4].copy_from_slice(pixel);
            return;
        }
    }

    fn load_palette(&mut self) {
        let new_palette = Some(self.get_bg_palette());
        self.current_palette = replace(&mut self.next_palette, new_palette);
    }

    fn load_tile(&mut self) {
        let new_tile = Some(self.get_bg_tile(self.read_vram((0x2000 | (self.internal_regs.v & 0xfff)) as usize)));
        self.current_tile = replace(&mut self.next_tile, new_tile);
    }

    fn get_bg_palette(&self) -> Palette {
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

    fn render_background_tiles(&mut self, x: u8) -> Option<&'static [u8; 4]> {
        let x_offset = x % 8 + self.internal_regs.get_fine_x();
        let tile = if x_offset > 7 { &mut self.next_tile } else { &mut self.current_tile };
        let palette = if x_offset > 7 { &mut self.next_palette } else { &mut self.current_palette };
        let brightness = tile.as_mut().unwrap().pixel_intensity(
            x_offset % 8,
            self.internal_regs.get_fine_y(),
        );

        if brightness > 0 {
            Some(
                palette
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
            if !(x >= sprite.x && x - sprite.x < 8) {
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

    fn sprite0_hit_detection(&mut self, x: u8, sprite_rendered: &SpriteInfo) {
        /* nothing to do if any of these hold (some of which are checked before this call):
         * -the hit sprite isn't sprite 0
         * -sprite zero has already been found
         * -we're not rendering both sprites and the background,
         * -we're checking a pixel on the far left of the screen and we don't have both
         *   background and sprite rendering enabled for that column,
         * -x is the rightmost column, 255
         */
        if sprite_rendered.sprite_index != 0 || self.ppu_status & (1 << 6) != 0 || x == 0xff {
            return;
        }
        /* check to see if we're already rendering a background pixel; if so, it's a hit */
        if self.current_tile.as_mut().unwrap().pixel_intensity(
            x % 8 + self.internal_regs.get_fine_x(),
            self.internal_regs.get_fine_y(),
        ) > 0
        {
            self.ppu_status |= 1 << 6;
        }
    }

    pub(super) fn sprite_height(&self) -> u8 {
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
    fn render_background_color(&self) -> &'static [u8; 4] {
        Palette::hue_lookup(self.palette_memory[0] as usize)
    }

    /* Finds the first eight sprites on the given scanline, determined
     * by position in the OAM. Takes into account whether sprites are 8 or 16
     * pixels tall. It then copies these into secondary OAM. Also sets the
     * sprite overflow bit if necessary.
     */
    fn sprite_evaluation(&mut self, scanline_num: u8) -> Vec<SpriteInfo> {
        let mut scanline_sprites = Vec::new();
        let mut sprites_found = 0;
        for i in 0..OAM_SIZE / 4 {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self.sprite_height()) {
                scanline_sprites.push(sprite_data);
                sprites_found += 1;
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                /* already found eight sprites, set overflow */
                if sprites_found == 8 {
                    set_bit_on(&mut self.ppu_status, 1);
                    break;
                }
            }
        }
        scanline_sprites
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam[sprite_index * 4..sprite_index * 4 + 4]);
        SpriteInfo::from_memory(sprite_index, &sprite_data)
    }

    pub(super) fn get_sprite_tile(&self, tile_index: u8) -> Tile {
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

    pub(super) fn get_palette(&self, palette_index: u8) -> Palette {
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

fn set_bit_on(flags: &mut u8, bit: u8) {
    *flags = *flags | (1 << bit);
}

fn set_bit_off(flags: &mut u8, bit: u8) {
    *flags = *flags & !(1 << bit);
}
