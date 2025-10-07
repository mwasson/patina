use crate::cpu::CoreMemory;
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
    memory: Rc<RefCell<CoreMemory>>,
    pub(super) mapper: Rc<RefCell<Box<dyn Mapper>>>,
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

#[derive(Debug)]
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

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn render_scanline(&mut self, scanline: u8) {
        let (mut foreground_sprites, mut background_sprites) = self.sprite_evaluation(scanline);

        if (scanline + OVERSCAN >= 240) {
            return;
        }

        /* technically should occur at the end of previous scanline, but if the entire scanline occurs
         * at once, this guarantees the CPU has updated its side of things
         */
        self.internal_regs.copy_x_bits();

        let render_background = self.ppu_mask & (1 << 3) != 0;
        let render_sprites = self.ppu_mask & (1 << 4) != 0;

        let mapper = self.mapper.borrow();
        let mut tile = self.get_current_tile(&mapper);
        let mut palette = self.palette_for_current_bg_tile();
        let mut index: usize = (scanline as usize) << 10;

        let fine_y = self.internal_regs.get_fine_y();



        let mut sprite0 = Some(self.slice_as_sprite(0));
        if !sprite0.as_ref().unwrap().in_scanline(scanline, self.sprite_height()) {
            sprite0 = None;
        }
        let mut sprite0_found = false;

        /* background color: grab the color directly without loading the full palette */
        let bg_color = self.palette_memory[0];

        /* this could be an inclusive range over all of the 8-bit address space, but inclusive ranges
         * don't output efficiently, even under optimization
         */
        let mut x = 0;
        loop {
            let pixel = 'pixel: {
                if let Some(pixel) = self.render_sprites(render_sprites, &mut foreground_sprites, scanline, x, &mapper) {
                    pixel
                } else if let Some(pixel) = self.render_background_tiles(render_background, scanline, x, fine_y,
                                                                         &mut tile, &palette,
                                                                         &mut sprite0, &mut sprite0_found,
                                                                         &mapper) {
                    pixel
                } else if let Some(pixel) = self.render_sprites(render_sprites, &mut background_sprites, scanline, x, &mapper) {
                    pixel
                } else {
                    bg_color
                }
            };

            PPU::render_pixel(&mut self.internal_buffer, index, Palette::load_color(pixel));
            index += 4;
            if x % 8 + self.internal_regs.get_fine_x() == 7 {
                /* increment coarse x; every other tile, also update the palette */
                if self.internal_regs.coarse_x_increment() % 2 == 0 {
                    palette = self.palette_for_current_bg_tile();
                }
                tile = self.get_current_tile(&mapper);
            }
            if x == 0xff {
                break;
            }
            x += 1;
        }

        if sprite0_found {
            self.ppu_status = set_bit_on(self.ppu_status, 6)
        }

        /* update scrolling */
        self.internal_regs.y_increment();
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn render_background_tiles(
        &self,
        render_background: bool,
        scanline: u8,
        x: u8,
        fine_y: u8,
        tile: &mut Tile,
        palette: &Palette,
        sprite0: &mut Option<SpriteInfo>,
        sprite0_found: &mut bool,
        mapper: &Box<dyn Mapper>,
    ) -> Option<u8> {
        if !render_background {
            return None;
        }
        let relative_x = x.wrapping_sub(self.internal_regs.get_tile_left()) % 8;

        let brightness = tile.pixel_intensity(mapper, relative_x, fine_y);

        if brightness > 0 {
            if !*sprite0_found && sprite0.is_some() && self.check_sprite0(scanline, x, sprite0.as_mut().unwrap(), mapper) {
                *sprite0_found = true;
            }
            Some(palette.data[brightness])
        } else {
            None
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn check_sprite0(&self, scanline: u8, x: u8, sprite0: &mut SpriteInfo, mapper: &Box<dyn Mapper>) -> bool {
        /* sprite zero hit detection */
        let x_offset = x.wrapping_sub(sprite0.x);
        if x_offset < 8 && sprite0.get_brightness_localized(self, mapper, x_offset, scanline - sprite0.y) > 0
        {
            true
        } else {
            false
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_current_tile(&self, mapper: &Box<dyn Mapper>) -> Tile {
        self.get_bg_tile(self.read_vram_with_mapper((0x2000 | (self.internal_regs.v & 0xfff)) as usize, mapper), mapper)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn render_sprites(
        &self,
        render_sprites: bool,
        scanline_sprites: &mut Vec<SpriteInfo>,
        scanline: u8,
        x: u8,
        mapper: &Box<dyn Mapper>) -> Option<u8> {
        if !render_sprites {
            return None;
        }
        for sprite in scanline_sprites {
            /* TODO doesn't render sprites on edges */
            let x_offset = x.wrapping_sub(sprite.x);
            if x_offset < 8 {
                let brightness =
                    sprite.get_brightness_localized(self, mapper, x_offset, scanline - sprite.y);
                if brightness > 0 {
                    let sprite_palette = sprite.get_palette(&self);
                    return Some(sprite_palette.data[brightness])
                };
            }
        }
        None
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn render_pixel(buffer:&mut [u8], pixel_index: usize, pixel : &[u8;4]) {
        /* seems more efficient to unroll this than create an index for small values */
        buffer[pixel_index] = pixel[0];
        buffer[pixel_index + 1] = pixel[1];
        buffer[pixel_index + 2] = pixel[2];
        buffer[pixel_index + 3] = pixel[3];
    }

    fn sprite_height(&self) -> u8 {
        if self.tall_sprites {
            16
        } else {
            8
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
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
    #[cfg_attr(feature = "profiling", inline(never))]
    fn sprite_evaluation(&mut self, scanline_num: u8) -> (Vec<SpriteInfo>, Vec<SpriteInfo>) {
        let mut foreground_sprites = Vec::new();
        let mut background_sprites = Vec::new();
        let mut sprites_found = 0;
        for i in 0..OAM_SIZE / 4 {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self.sprite_height()) {
                /* already found eight sprites, set overflow */
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                if sprites_found >= 8 {
                    self.ppu_status = set_bit_on(self.ppu_status, 1);
                    break;
                }
                sprites_found += 1;
                if sprite_data.is_foreground() {
                    foreground_sprites.push(sprite_data);
                } else {
                    background_sprites.push(sprite_data);
                }
            }
        }
        (foreground_sprites, background_sprites)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam[sprite_index * 4..sprite_index * 4 + 4]);
        SpriteInfo::from_memory(&sprite_data)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_sprite_tile(&self, tile_index: u8, mapper: &Box<dyn Mapper>) -> Tile {
        let (tile_index, pattern_table) = if self.tall_sprites {
            (tile_index & !1, tile_index & 1)
        } else {
            (tile_index, (self.ppu_ctrl & 0x8) >> 3)
        };
        self.get_tile(tile_index, pattern_table, mapper)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_bg_tile(&self, tile_index: u8, mapper: &Box<dyn Mapper>) -> Tile {
        self.get_tile(tile_index, (self.ppu_ctrl & 0x10) >> 4, mapper)
    }

    fn get_tile(&self, tile_index: u8, pattern_table_num: u8, mapper: &Box<dyn Mapper>) -> Tile {
        mapper.read_tile(tile_index, pattern_table_num)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc: usize = (palette_index as usize) * 4;

        Palette::new(&self.palette_memory[palette_mem_loc..palette_mem_loc + 4])
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn read_vram(&self, addr: usize) -> u8 {
        self.read_vram_with_mapper(addr, &self.mapper.borrow())
    }

    pub fn read_vram_with_mapper(&self, addr: usize, mapper: &Box<dyn Mapper>) -> u8 {
        let mapped_address = self.vram_address_mirror(addr, mapper);

        /* pattern tables (CHR data) */
        if mapped_address < 0x2000 {
            mapper.read_chr(mapped_address as u16)
        /* nametables and attribute tables */
        } else if mapped_address < 0x3000 {
            self.vram[mapped_address - 0x2000]
        /* palettes */
        } else {
            self.palette_memory[mapped_address - 0x3f00]
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn write_vram(&mut self, addr: usize, val: u8) {
        let mapped_address = self.vram_address_mirror(addr, &self.mapper.borrow());

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

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn vram_address_mirror(&self, addr: usize, mapper: &Box<dyn Mapper>) -> usize {
        let mut result = addr;

        if result < 0x2000 {
            result
        } else if result < 0x3f00 {
            if result >= 0x3000 {
                result -= 0x1000;
            }

            let foo = Some(NametableMirroring::Horizontal);

            /* TODO document */
            match mapper.get_nametable_mirroring() {
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
             * this matters especially for 0x3f10, the first sprite palette's background color,
             * which some games write the overall background color to
             */
            /* 10 => 00; 14 => 04; 18 -> 08 */
            if result & 0x3 == 0 {
                result &= !0xf0;
            }

            result
        }
    }
}

#[derive(Debug)]
struct SpriteInfo {
    /* NB: this is one less than the top of the sprite! you'll have to add 1 whenever you use it (see get_y) */
    y: u8,
    tile_index: u8,
    attrs: u8,
    x: u8,
    cached_tile: Option<Tile>,
    cached_palette: Option<Palette>,
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, sprite_height: u8) -> bool {
        self.y <= scanline && scanline - self.y < sprite_height
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_brightness_localized(&mut self, ppu: &PPU, mapper: &Box<dyn Mapper>, x: u8, y: u8) -> usize {
        if self.cached_tile.is_none() {
            self.cached_tile = Some(ppu.get_sprite_tile(self.tile_index, mapper));
        }
        let tile: &mut Tile = self.cached_tile.as_mut().unwrap();
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
        tile.pixel_intensity(mapper, x_to_use, y_to_use)
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    fn get_palette(&mut self, ppu: &PPU) -> &Palette {
        if self.cached_palette.is_none() {
            self.cached_palette = Some(ppu.get_palette((self.attrs & 0x3) + 4));
        }
        self.cached_palette.as_ref().unwrap()
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x20 == 0
    }

    /* create a SpriteInfo from memory */
    fn from_memory(src_slice: &[u8]) -> SpriteInfo {
        SpriteInfo {
            y: src_slice[0]+1,
            tile_index: src_slice[1],
            attrs: src_slice[2],
            x: src_slice[3],
            cached_tile: None,
            cached_palette: None,
        }
    }
}

fn set_bit_on(flags: u8, bit: u8) -> u8 {
    flags | (1 << bit)
}

fn set_bit_off(flags: u8, bit: u8) -> u8 {
    flags & !(1 << bit)
}
