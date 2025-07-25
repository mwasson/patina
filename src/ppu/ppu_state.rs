use std::ops::Deref;
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
            let test_start = Instant::now();
            /* visible scanlines */
            for i in (0..240) {
                let scanline_pixels = &ppu.render_scanline(i);
                ppu.write_buffer.lock().unwrap()[Self::pixel_range_for_line(i)].copy_from_slice(scanline_pixels);
            }
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
        // println!("Screen rendering time: {}ms", Instant::now().duration_since(start_time).as_millis());
        self.print_vram_memory(0x23f0, 0x16);
    }

    pub fn get_write_buffer(&self) -> Arc<Mutex<WriteBuffer>> {
        self.write_buffer.clone()
    }

    fn render_scanline(&mut self, scanline: u8) -> [u8; 256*4]{
        // println!("Rendering line {scanline}");
        for i in (0x23f0..0x2400)
        {
            // if i % 8 >= 4 {
            //     let new_val = self.vram.lock().unwrap()[i - 4];
            //     self.vram.lock().unwrap()[i] = new_val;
            // }
        }

        let scanline_sprites = self.sprite_evaluation(scanline);

        let mut line_buffer = [0; 256*4];
        let line_buffer_num_pixels = line_buffer.len()/4;
        for (pixel_buffer,i) in line_buffer.chunks_exact_mut(4).zip((0..line_buffer_num_pixels)) {
            let iu8 = i as u8;
            let (fg_sprite, fg_brightness) = self.find_first_sprite(iu8, scanline, true, &scanline_sprites);
            let (bg_pixels, bg_tile_brightness) = self.background_brightness(iu8, scanline);
            let (bg_sprite, bg_sprite_brightness) = self.find_first_sprite(iu8, scanline, false, &scanline_sprites);

            /* check for sprite zero hits */
            if fg_brightness > 0 && bg_tile_brightness > 0 && fg_sprite.unwrap().sprite_index == 0 {
                PPUSTATUS.set_flag_on(&mut self.memory, 6);
            }

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

        line_buffer
    }

    fn find_first_sprite(&self, x: u8, y: u8, is_foreground: bool, scanline_sprites: &Vec<SpriteInfo>) -> (Option<SpriteInfo>, u8) {
        let mut brightness = 0;
        let mut sprite = None;
        for cur_sprite in scanline_sprites {
            if cur_sprite.at_x_position(x) && cur_sprite.is_foreground() == is_foreground {
                let sprite_brightness = cur_sprite.get_brightness(self, x, y);
                if(sprite_brightness != 0) {
                    brightness = sprite_brightness;
                    sprite = Some(cur_sprite.clone());
                    break;
                }
            }
        }
        (sprite, brightness)
    }

    fn background_brightness(&self, x:u8, y:u8) -> ([u8;4], u8) {
        let tile = self.tile_for_pixel(x,y);
        let palette = self.palette_for_pixel(x,y);

        let brightness = tile.pixel_intensity((x % 8) as usize, (y % 8) as usize);

        (palette.brightness_to_pixels(brightness), brightness)
    }

    /* TODO; this only uses the first name table */
    fn tile_for_pixel(&self, x:u8, y:u8) -> Tile {
        let nametable_base_addr : usize = 0x2000 + 0x400 * (PPUCTRL.read(&self.memory) & 0x3) as usize;
        let offset : usize = (y as usize)/8*32 + (x as usize)/8;
        let tile_index = self.vram.lock().unwrap()[nametable_base_addr + offset];
        self.get_bg_tile(tile_index)
    }

    /* TODO: this only uses the first attribute table */
    fn palette_for_pixel(&self, x:u8, y:u8) -> Palette {
        let nametable_base = 0x2000 + 0x400 * (PPUCTRL.read(&self.memory) & 0x3) as usize;
        /* each address controls a 32x32 pixel block; 8 blocks per row */
        let attr_addr = y/32*8 + x/32;
        let attr_table_value = self.vram.lock().unwrap()[nametable_base + 0x3c0 + attr_addr as usize];
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
    fn get_sprite_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, ((PPUCTRL.read(&self.memory) & 0x8) >> 3) as usize)
    }

    fn get_bg_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, ((PPUCTRL.read(&self.memory) & 0x10) >> 4) as usize)
    }

    fn get_tile(&self, tile_index: u8, pattern_table_num: usize) -> Tile {
        let pattern_table_base : usize = 0x1000 * pattern_table_num;
        let tile_start = pattern_table_base + (tile_index as usize * 16);
        let mut memcopy = [0u8; 16];
        memcopy.copy_from_slice(&self.vram.lock().unwrap()[tile_start..tile_start+16]);
        Tile::from_memory(memcopy)
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc : usize = 0x3f00 + (palette_index as usize)*4;
        let mut palette_data = [0u8; 4];
        palette_data.copy_from_slice(&self.vram.lock().unwrap()[palette_mem_loc..palette_mem_loc+4]);

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
            self.y <= scanline &&  scanline - self.y < 8 /* TODO ppu.sprite_size() */
    }

    fn at_x_position(&self, x: u8) -> bool {
        self.x <= x && x - self.x < 8
    }

    fn get_brightness(&self, ppu: &PPUState, x:u8, y:u8) -> u8 {
        self.get_brightness_localized(ppu,x-self.x,y-self.y)
    }

    fn get_brightness_localized(&self, ppu: &PPUState, x:u8, y:u8) -> u8 {
        let tile = ppu.get_sprite_tile(self.tile_index); /* TODO is this right? */
        tile.pixel_intensity(x as usize, y as usize)
    }

    fn color_from_brightness(&self, ppu: &PPUState, brightness: u8) -> [u8; 4] {
        self.get_palette(&ppu).brightness_to_pixels(brightness)
    }

    fn get_palette(&self, ppu: &PPUState) -> Palette {
        ppu.get_palette(self.attrs & 0x3)
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