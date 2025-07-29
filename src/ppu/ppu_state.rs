use std::sync::{Arc, Mutex};
use std::time::Instant;
use crate::cpu::{CoreMemory};
use crate::rom::Rom;

use crate::ppu::palette::Palette;
use crate::ppu::ppu_listener::PPUListener;
use crate::ppu::ppu_registers::PPURegister::{PPUCTRL, PPUSTATUS};
use crate::ppu::{Tile, WriteBuffer, OAM, OAM_SIZE, PPU_MEMORY_SIZE, VRAM, WRITE_BUFFER_SIZE};
use crate::processor::Processor;

/* TODO: ppumask rendering effects */

/* TODO ppuscroll scrolling */

/* TODO PPU internal registers */

pub struct PPUState {
    vram: Arc<Mutex<VRAM>>,
    oam: Arc<Mutex<OAM>>,
    memory: CoreMemory,
    write_buffer: Arc<Mutex<WriteBuffer>>,
    ppu_internal_registers: Arc<Mutex<PPUInternalRegisters>>
}

pub struct PPUInternalRegisters
{
    /* rendering: scroll position; otherwise: current vram address */
    pub v: u16,
    /* rendering: starting coarse-x scroll for scanline, y scroll for screen;
     * otherwise: holds data to transfer to v */
    pub t: u16,
    /* fine-x position of current scroll */
    pub x: u8,
    /* toggles on write to PPUSCROLL or PPUADDR, indicating whether 1st/2nd write; 'write latch' */
    pub w: u8,
    /* buffer for reading via PPUDATA */
    pub read_buffer: u8
}

impl Processor for PPUState {
    fn clock_speed(&self) -> u64 {
        1790000*3 /* 3x as fast as the CPU */
    }
}

impl PPUState {

    pub fn from_rom(rom: &Rom, memory: CoreMemory) -> Box<PPUState> {
        let mut vram: [u8; PPU_MEMORY_SIZE] = [0; PPU_MEMORY_SIZE];
        let oam : [u8; OAM_SIZE] = [0; OAM_SIZE]; /* TODO: link this to CPU memory? */

        /* copy over character data; TODO surely this is not correct even in the no-mapper case*/
        vram[0x0000..rom.chr_data.len()].copy_from_slice(&rom.chr_data);

        let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));

       Box::new(PPUState {
            vram: Arc::new(Mutex::new(vram)),
            oam: Arc::new(Mutex::new(oam)),
            memory,
            write_buffer,
            ppu_internal_registers: Arc::new(Mutex::new(PPUInternalRegisters {
                v: 0,
                t: 0,
                x: 0,
                w: 0,
                read_buffer: 0,
            }))
        })
    }

    pub fn get_listener(&self) -> PPUListener {
        PPUListener::new(&self.vram, &self.oam, &self.ppu_internal_registers, &self.memory)
    }

    pub fn render_screen(&mut self) { ;
        let start_time = Instant::now();
        /* dummy scanline */
        /* clear VBlank */
        PPUSTATUS.set_flag_off(&mut self.memory, 7);
        /* clear sprite 0 hit flag */
        PPUSTATUS.set_flag_off(&mut self.memory, 6);
        self.run_timed(341, |_unused| {});

        self.run_timed(341*240, |ppu| {
            /* slightly faster to avoid either constantly locking or locking for a long time */
            let mut tmp_write_buffer = [0; WRITE_BUFFER_SIZE];
            /* visible scanlines */
            for i in (0..240) {
                let test_start = Instant::now();
                let scanline_pixels = &ppu.render_scanline_take_2(i);
                tmp_write_buffer[Self::pixel_range_for_line(i)].copy_from_slice(scanline_pixels);
            }
            ppu.write_buffer.lock().unwrap().copy_from_slice(&tmp_write_buffer);
        });

        /* post-render scanline; first tick of VBlank */
        self.run_timed(341, |_unused| {});

        /* set vblank flag */
        PPUSTATUS.set_flag_on(&mut self.memory, 7);
        /* vblank NMI */
        if PPUCTRL.read_flag(&mut self.memory, 7) {
            self.memory.trigger_nmi();
        }

        /* VBlank scanlines */
        self.run_timed(20*341-2, |_unused| {});
    }

    pub fn get_write_buffer(&self) -> Arc<Mutex<WriteBuffer>> {
        self.write_buffer.clone()
    }

    fn render_scanline_take_2(&mut self, scanline: u8) -> [u8; 256*4] {
        let scanline_sprites = self.sprite_evaluation(scanline);
        let ppuctrl = PPUCTRL.read(&self.memory);

        let mut line_buffer = [0; 256*4];

        let _ = self.vram.lock().and_then(|vram| {
            self.render_sprites(&scanline_sprites, &vram, ppuctrl, scanline, &mut line_buffer, true);
            /* background tiles */
            for x in (0..0xff).step_by(8) {
                let tile = self.tile_for_pixel(&vram, ppuctrl, x, scanline);
                let palette = self.palette_for_pixel(&vram, ppuctrl, x, scanline);
                for pixel_offset in 0..8 {
                    let brightness = tile.pixel_intensity(pixel_offset as usize, (scanline % 8) as usize);
                    let index = (x+pixel_offset) as usize * 4;
                    if brightness > 0 && line_buffer[index+3] == 0 {
                        /* TODO doesn't handle 16 pixel tall sprites */
                        line_buffer[(index)..(index+4)].copy_from_slice(&palette.brightness_to_pixels(brightness));
                    }
                }
            }
            self.render_sprites(&scanline_sprites, &vram, ppuctrl, scanline, &mut line_buffer, false);

            /* background color */
            let bg_pixels = self.get_palette_no_locking(&vram, 0).brightness_to_pixels(0);
            for x in (0..0x100) {
                if line_buffer[x*4 + 3] == 0 {
                    line_buffer[(x*4)..(x*4+4)].copy_from_slice(&bg_pixels);
                }
            }

            Ok(())
        });

        self.check_for_sprite_zero_hit(scanline, ppuctrl);

        line_buffer
    }

    fn check_for_sprite_zero_hit(&mut self, scanline: u8, ppuctrl: u8) {
        let vram = self.vram.lock().unwrap();
        /* TODO: don't do anything if it's already been set, should be a ppu temp local variable */
        /* TODO it'd be nice if unlocked vram and ppuctrl and maybe others could just be ppu local variables ... */
        let sprite0 = self.slice_as_sprite(0);
        if sprite0.in_scanline(scanline, self) {
            for x in 0..8 {
                if sprite0.get_brightness_localized_no_locking(&vram, ppuctrl, self, x, scanline - sprite0.y) > 0 {
                    let bg_tile = self.tile_for_pixel(&vram, ppuctrl, sprite0.x + x, scanline);
                    if  bg_tile.pixel_intensity(((sprite0.x + x) % 8) as usize, (scanline % 8) as usize) > 0 {
                        PPUSTATUS.set_flag_on(&mut self.memory, 6);
                        return;
                    }
                }
            }
        }
    }

    fn render_sprites(&self, scanline_sprites: &Vec<SpriteInfo>, vram: &VRAM, ppuctrl: u8, scanline: u8, line_buffer: &mut [u8], is_foreground: bool) {
        for sprite in scanline_sprites {
            if(sprite.is_foreground() != is_foreground) {
                continue;
            }
            let sprite_palette = sprite.get_palette_no_locking(&vram, &self);
            /* sprites */
            for i in (0..8) {
                let brightness = sprite.get_brightness_localized_no_locking(&vram, ppuctrl, self, i, scanline - sprite.y);
                let pixel_index = sprite.x.wrapping_add(i) as usize * 4;
                if(brightness > 0 && line_buffer[pixel_index+3] == 0) {
                    line_buffer[pixel_index..pixel_index+4].copy_from_slice(&sprite_palette.brightness_to_pixels(brightness));
                }
            }
        }
    }

    fn find_first_sprite(&self, vram: &VRAM, ppuctrl: u8, x: u8, y: u8, is_foreground: bool, scanline_sprites: &Vec<SpriteInfo>) -> (Option<SpriteInfo>, u8) {
        let mut brightness = 0;
        let mut sprite = None;
        for cur_sprite in scanline_sprites {
            if cur_sprite.at_x_position(x) && cur_sprite.is_foreground() == is_foreground {
                let sprite_brightness = cur_sprite.get_brightness_no_locking(vram, ppuctrl, self, x, y);
                if(sprite_brightness != 0) {
                    brightness = sprite_brightness;
                    sprite = Some(cur_sprite.clone());
                    break;
                }
            }
        }
        (sprite, brightness)
    }

    fn background_brightness(&self, vram: &VRAM, ppuctrl:u8, x:u8, y:u8) -> ([u8;4], u8) {
        let tile = self.tile_for_pixel(vram, ppuctrl, x, y);
        let palette = self.palette_for_pixel(&vram, ppuctrl, x, y);
        let brightness = tile.pixel_intensity((x % 8) as usize, (y % 8) as usize);
        (palette.brightness_to_pixels(brightness), brightness)
    }

    fn tile_for_pixel(&self, vram: &VRAM, ppuctrl: u8, x:u8, y:u8) -> Tile {
        let nametable_base_addr : usize = 0x2000 + 0x400 * ((ppuctrl & 0x3) as usize);
        let offset : usize = (y as usize)/8*32 + (x as usize)/8;
        /* TODO implement vram mirroring */
        let tile_index = vram[nametable_base_addr + offset];
        self.get_bg_tile(vram, tile_index, ppuctrl)
    }

    fn palette_for_pixel(&self, vram: &VRAM, ppuctrl: u8, x:u8, y:u8) -> Palette {
        let nametable_base = 0x2000 + 0x400 * (ppuctrl as u16 & 0x3);
        /* each address controls a 32x32 pixel block; 8 blocks per row */
        let attr_addr = y/32*8 + x/32;
        let attr_table_value = vram[Self::vram_address_mirror(nametable_base + 0x3c0 + attr_addr as u16)];
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
        self.get_palette_no_locking(vram,(attr_table_value >> attr_table_offset) & 3) /* only need two bits */
    }

    /* Finds the first eight sprites on the given scanline, determined
     * by position in the OAM. Takes into account whether sprites are 8 or 16
     * pixels tall. It then copies these into secondary OAM. Also sets the
     * sprite overflow bit if necessary.
     */
    fn sprite_evaluation(&mut self, scanline_num: u8) -> Vec<SpriteInfo>{
        let sprite_size = 8; /* TODO */
        let mut scanline_sprites = Vec::new();
        for i in (0..OAM_SIZE/4) {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self) {
                /* already found eight sprites, set overflow */
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                if(scanline_sprites.len() >= 8) {
                    let old_status = PPUSTATUS.read(&self.memory);
                    PPUSTATUS.write(&mut self.memory, old_status | 0x20);
                    break;
                }
                scanline_sprites.push(sprite_data);
            }
        }
        scanline_sprites
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam.lock().unwrap()[sprite_index*4..sprite_index*4+4]);
        SpriteInfo::from_memory(&sprite_data, sprite_index as u8)
    }

    /* sprites are 8 pixels tall unless the 5th bit of PPUCTRL is true, then they're 16 */
    fn sprite_size(&self) -> u8 {
        if PPUCTRL.read(&self.memory) & 0x10 != 0 { 16 } else { 8 }
    }

    /* TODO: handle 8x16 sprites */
    fn get_sprite_tile(&self, vram: &VRAM, ppuctrl: u8, tile_index: u8) -> Tile {
        self.get_tile(vram, tile_index, ((ppuctrl & 0x8) >> 3) as usize)
    }

    fn get_bg_tile(&self, vram: &VRAM, tile_index: u8, ppuctrl: u8) -> Tile {
        self.get_tile(vram, tile_index, ((ppuctrl & 0x10) >> 4) as usize)
    }

    fn get_tile(&self, vram: &VRAM, tile_index: u8, pattern_table_num: usize) -> Tile {
        let pattern_table_base : usize = 0x1000 * pattern_table_num;
        let tile_start = pattern_table_base + (tile_index as usize * 16);
        let mut memcopy = [0u8; 16];
        memcopy.copy_from_slice(&vram[tile_start..tile_start+16]);
        Tile::from_memory(memcopy)
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        self.get_palette_no_locking(&self.vram.lock().unwrap(), palette_index)
    }

    fn get_palette_no_locking(&self, vram: &VRAM, palette_index: u8) -> Palette {
        let palette_mem_loc : usize = 0x3f00 + (palette_index as usize)*4;
        let mut palette_data = [0u8; 4];
        palette_data.copy_from_slice(&vram[palette_mem_loc..palette_mem_loc+4]);

        Palette::new(palette_data)
    }

    fn pixel_range_for_line(x: u8) -> core::ops::Range<usize> {
        let x_usize = x as usize;
        let range_width = 4*256; /* 4 bytes per pixel, 256 pixels per line */
        ((x_usize*range_width)..(x_usize+1)*range_width)
    }

    fn print_nametable(&self) {
        let nametable_size = 0x400;
        let nametable_base = 0x2400 + ((PPUCTRL.read(&self.memory) as usize ) & 0x3) * 0x400;
        self.print_vram_memory(nametable_base, nametable_size);
    }

    pub fn print_vram_memory(&self, base_addr: usize, len: usize) {
        let mut mem = Vec::new();
        mem.extend_from_slice(&self.vram.lock().unwrap()[base_addr..(base_addr+len)]);
        let mut output = String::new();
        output.push_str(format!("VRAM Memory [0x{:x}..0x{:x}] : \n", base_addr, base_addr+len).as_str());
        for i in 0..len {
            output.push_str(format!("[0x{:x}]: 0x{:x}\n", base_addr+i, mem[i]).as_str());
        }
        println!("{}", output);
    }

    pub fn vram_address_mirror(addr: u16) -> usize {
        let mut result = addr;

        /* palettes are repeated above 0x3f1f */
        if result > 0x3f1f {
            result = 0x3f00 | (result & 0xff);
        }
        /* the first color of corresponding background and sprite palettes are shared;
         * this doesn't have any real effect, except if the true background color is
         * written to at 0x3f10
         */
        if result & 0xfff0 == 0x3f10 && result % 4 == 0 {
            result -= 0x10;
        }

        result as usize
    }
}

#[derive(Debug, Clone, Copy)]
struct SpriteInfo
{
    y: u8,
    tile_index: u8,
    attrs: u8,
    x: u8,
    sprite_index: u8,
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, ppu: &PPUState) -> bool {
            self.y <= scanline && scanline - self.y < 8 /* TODO ppu.sprite_size() */
    }

    fn at_x_position(&self, x: u8) -> bool {
        self.x <= x && x - self.x < 8
    }
    fn get_brightness_no_locking(&self, vram: &VRAM, ppuctrl: u8, ppu: &PPUState, x: u8, y: u8) -> u8 {
        self.get_brightness_localized_no_locking(vram, ppuctrl, ppu, x-self.x,y-self.y)
    }

    fn get_brightness_localized_no_locking(&self, vram: &VRAM, ppuctrl: u8, ppu: &PPUState, x: u8, y: u8) -> u8 {
        let tile = ppu.get_sprite_tile(vram, ppuctrl, self.tile_index); /* TODO is this right? */
        let mut x_to_use = x as usize;
        if self.attrs & 0x40 != 0 { /* flipped horizontally */
            x_to_use = 7-x_to_use;
        }
        let mut y_to_use = y as usize;
        if self.attrs & 0x80 != 0 { /* flipped horizontally */
            y_to_use = 7-y_to_use;
        }
        tile.pixel_intensity(x_to_use, y_to_use)
    }

    fn color_from_brightness(&self, vram: &VRAM, ppu: &PPUState, brightness: u8) -> [u8; 4] {
        ppu.get_palette_no_locking(vram,self.attrs & 0x3).brightness_to_pixels(brightness)
    }

    fn get_palette(&self, ppu: &PPUState) -> Palette {
        ppu.get_palette(self.attrs & 0x3)
    }

    fn get_palette_no_locking(&self, vram: &VRAM, ppu: &PPUState) -> Palette {
        ppu.get_palette_no_locking(&vram, self.attrs & 0x3)
    }

    /* write this sprite as a byte array into memory */
    fn copy_to_mem(&self, dst_slice: &mut [u8]) {
        dst_slice[0] = self.y;
        dst_slice[1] = self.tile_index;
        dst_slice[2] = self.attrs;
        dst_slice[3] = self.x;
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x10 == 0
    }

    /* create a SpriteInfo from memory */
    fn from_memory(src_slice: &[u8], index: u8) -> SpriteInfo {
        SpriteInfo {
            y: src_slice[0],
            tile_index: src_slice[1],
            attrs: src_slice[2],
            x: src_slice[3],
            sprite_index: index,
        }
    }
}