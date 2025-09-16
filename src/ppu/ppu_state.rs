use std::cmp::{max, min};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use crate::rom::Rom;

use crate::ppu::palette::Palette;
use crate::ppu::{PPUScrollState, Tile, WriteBuffer, OAM, OAM_SIZE, PPU_MEMORY_SIZE, VRAM, WRITE_BUFFER_SIZE};
use crate::cpu::cpu_to_ppu_message::CpuToPpuMessage;
use crate::ppu::ppu_to_cpu_message::PpuToCpuMessage;
use crate::ppu::ppu_to_cpu_message::PpuToCpuMessage::{PpuStatus, NMI};
use crate::processor::Processor;

pub struct PPUState {
    vram: VRAM,
    oam: OAM,
    write_buffer: Arc<Mutex<WriteBuffer>>,
    internal_buffer: WriteBuffer,
    /* shared registers */
    ppu_ctrl: u8,
    ppu_mask: u8,
    ppu_status: u8,
    scroll: PPUScrollState,
    tmp_scroll: PPUScrollState,
    /* communication back to cpu */
    ppu_update_sender: Sender<PpuToCpuMessage>,
    update_receiver: Receiver<CpuToPpuMessage>
}

impl Processor for PPUState {
    fn clock_speed(&self) -> u64 {
        1790000*3 /* 3x as fast as the CPU */
    }
}

impl PPUState {

    pub fn from_rom(rom: &Rom, ppu_update_sender: Sender<PpuToCpuMessage>, update_receiver: Receiver<CpuToPpuMessage>) -> Box<PPUState> {
        let mut vram: [u8; PPU_MEMORY_SIZE] = [0; PPU_MEMORY_SIZE];
        let oam : [u8; OAM_SIZE] = [0; OAM_SIZE]; /* TODO: link this to CPU memory? */

        /* copy over character data; TODO surely this is not correct even in the no-mapper case*/
        vram[0x0000..rom.chr_data.len()].copy_from_slice(&rom.chr_data);

        let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));

       Box::new(PPUState {
           vram,
           oam,
           write_buffer,
           internal_buffer: [0; WRITE_BUFFER_SIZE],
           ppu_status: 0,
           ppu_ctrl: 0,
           ppu_mask: 0,
           ppu_update_sender,
           update_receiver,
           scroll: PPUScrollState::default(),
           tmp_scroll: PPUScrollState::default(),
       })
    }

    pub fn beginning_of_screen_render(&mut self) {
        /* dummy scanline */
        /* clear VBlank */
        self.ppu_status = set_bit_off(self.ppu_status, 7);
        /* clear sprite 0 hit flag */
        self.ppu_status = set_bit_off(self.ppu_status, 6);
        /* clear overflow flag */
        self.ppu_status = set_bit_off(self.ppu_status, 5);
        self.send_status_update();
    }

    pub fn end_of_screen_render(&mut self) {
        self.handle_update();
        /* set vblank flag */
        self.ppu_status = set_bit_on(self.ppu_status, 7);
        self.send_status_update();
        /* vblank NMI */
        if self.ppu_ctrl & (1 << 7) != 0 {
            self.send_update(NMI);
        }

        /* write new pixels so UI can see them */
        self.write_buffer.lock().unwrap().copy_from_slice(&self.internal_buffer);
    }

    pub fn get_write_buffer(&self) -> Arc<Mutex<WriteBuffer>> {
        self.write_buffer.clone()
    }

    pub fn render_scanline(&mut self, scanline: u8) {
        self.handle_update();
        if scanline == 0 {
            self.tmp_scroll = self.scroll.clone();
        } else {
            self.tmp_scroll.coarse_x = self.scroll.coarse_x;
            self.tmp_scroll.nametable = (self.tmp_scroll.nametable & !1) | (self.scroll.nametable & 1);
        }

        let scanline_sprites = self.sprite_evaluation(scanline);

        let mut line_buffer = [0; 256*4];

        let render_background = self.ppu_mask & (1 << 3) != 0;
        let render_sprites = self.ppu_mask & (1 << 4) != 0;

        if render_sprites {
            self.render_sprites(&scanline_sprites, scanline, &mut line_buffer, true);
        }
        /* background tiles */
        if render_background {
            for x in (0..0x101).step_by(8) {
                let tile = self.get_bg_tile(self.vram[Self::vram_address_mirror((0x2000
                    | (self.tmp_scroll.nametable as u16) << 10
                    | (self.tmp_scroll.coarse_y as u16) << 5
                    | (self.tmp_scroll.coarse_x as u16)) as usize)]);
                let palette = self.palette_for_pixel(self.tmp_scroll.coarse_x * 8, scanline);
                for pixel_offset in 0..8 {
                    let pixel_loc = x as i16 + pixel_offset as i16 - self.scroll.fine_x as i16;
                    if (pixel_loc < 0 || pixel_loc > 0xff) {
                        continue;
                    }
                    let brightness = tile.pixel_intensity(pixel_offset as usize,
                                                          self.tmp_scroll.fine_y as usize);

                    let index = pixel_loc as usize * 4;
                    if brightness > 0 && line_buffer[index + 3] == 0 {
                        /* TODO doesn't handle 16 pixel tall sprites */
                        line_buffer[index..(index + 4)].copy_from_slice(&palette.brightness_to_pixels(brightness));
                    }
                }

                self.tmp_scroll.coarse_x_increment();
            }
        }
        if render_sprites {
            self.render_sprites(&scanline_sprites, scanline, &mut line_buffer, false);
        }

        /* background color */
        let bg_pixels = self.get_palette(0).brightness_to_pixels(0);
        for x in 0..0x100 {
            if line_buffer[x*4 + 3] == 0 {
                line_buffer[(x*4)..(x*4+4)].copy_from_slice(&bg_pixels);
            }
        }

        self.check_for_sprite_zero_hit(scanline);

        /* update scrolling TODO explain */
        self.tmp_scroll.y_increment();

        self.internal_buffer[Self::pixel_range_for_line(scanline)].copy_from_slice(&line_buffer);
    }

    fn check_for_sprite_zero_hit(&mut self, scanline: u8) {
        /* if already set, return */
        if self.ppu_status & (1<<6) != 0 {
            return;
        }

        let mut scroll_data = self.tmp_scroll.clone();
        scroll_data.coarse_x = self.scroll.coarse_x;
        scroll_data.nametable = self.scroll.nametable;
        let sprite0 = self.slice_as_sprite(0);
        if sprite0.in_scanline(scanline, self) {
            let mut x_val = sprite0.x;
            while x_val >= 8 {
                scroll_data.coarse_x_increment();
                x_val -= 8;
            }
            for x in 0..8 {
                if x_val + x >= 8 {
                    scroll_data.coarse_x_increment();
                }
                if sprite0.get_brightness_localized(self, x, scanline - sprite0.get_y()) > 0 {
                    let bg_tile = self.get_bg_tile(self.vram[Self::vram_address_mirror((0x2000
                        | (scroll_data.nametable as u16) << 10
                        | (scroll_data.coarse_y as u16) << 5
                        | (scroll_data.coarse_x as u16)) as usize)]);
                    if  bg_tile.pixel_intensity(((sprite0.x + x) % 8) as usize, (scanline % 8) as usize) > 0 {
                        self.ppu_status = set_bit_on(self.ppu_status, 6);
                        self.send_status_update();
                        return;
                    }
                }
            }
        }
    }

    fn render_sprites(&self, scanline_sprites: &Vec<SpriteInfo>, scanline: u8, line_buffer: &mut [u8], is_foreground: bool) {
        for sprite in scanline_sprites {
            if sprite.is_foreground() != is_foreground {
                continue;
            }
            let sprite_palette = sprite.get_palette(&self);
            /* sprites */
            for i in 0..min(8,(0xff-sprite.x).saturating_add(1)) {
                let brightness = sprite.get_brightness_localized(self, i, scanline - sprite.get_y());
                let pixel_index = sprite.x.wrapping_add(i) as usize * 4;
                let pixels = sprite_palette.brightness_to_pixels(brightness);
                if brightness > 0 && line_buffer[pixel_index+3] == 0 {
                    line_buffer[pixel_index..pixel_index+4].copy_from_slice(&pixels);
                }
            }
        }
    }

    fn tile_for_pixel(&self, x:u8, y:u8) -> Tile {
        let nametable_base_addr : usize = 0x2000 + 0x400 * ((self.ppu_ctrl & 0x3) as usize);
        let offset : usize = (y as usize)/8*32 + (x as usize)/8;
        /* TODO implement vram mirroring */
        let tile_index = self.vram[nametable_base_addr + offset];
        self.get_bg_tile(tile_index)
    }

    fn palette_for_pixel(&self, x:u8, y:u8) -> Palette {
        /* TODO comment */
        /* 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07) */
        let addr = 0x23c0
            | ((self.tmp_scroll.nametable as u16) << 10)
            | (((self.tmp_scroll.coarse_y as u16) & 0x1c) << 1)
            | (((self.tmp_scroll.coarse_x) as u16) >> 2);
        /* each address controls a 32x32 pixel block; 8 blocks per row */
        let attr_table_value = self.vram[Self::vram_address_mirror(addr as usize)];
        /* the attr_table_value stores information about 16x16 blocks as 2-bit palette references.
         * in order from the lowest bits they are: upper left, upper right, bottom left, bottom right
         */
        let x_low = x % 32 < 16;
        let y_low = y % 32 < 16;
        let attr_table_offset =
            if x_low {
                if y_low { 0 } else { 4 }
            } else {
                if y_low { 2 } else { 6 }
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
        for i in 0..OAM_SIZE/4 {
            let sprite_data = self.slice_as_sprite(i);
            if sprite_data.in_scanline(scanline_num, self) {
                /* already found eight sprites, set overflow */
                /* TODO: should we implement the buggy 'diagonal' behavior for this? */
                if scanline_sprites.len() >= 8 {
                    self.ppu_status = set_bit_on(self.ppu_status, 1);
                    self.send_status_update();
                    break;
                }
                scanline_sprites.push(sprite_data);
            }
        }
        scanline_sprites
    }

    fn slice_as_sprite(&self, sprite_index: usize) -> SpriteInfo {
        let mut sprite_data = [0u8; 4];
        sprite_data.copy_from_slice(&self.oam[sprite_index*4..sprite_index*4+4]);
        SpriteInfo::from_memory(&sprite_data, sprite_index as u8)
    }

    /* sprites are 8 pixels tall unless the 5th bit of PPUCTRL is true, then they're 16 */
    fn sprite_size(&self) -> u8 {
        if self.ppu_ctrl & 0x10 != 0 { 16 } else { 8 }
    }

    /* TODO: handle 8x16 sprites */
    fn get_sprite_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, ((self.ppu_ctrl & 0x8) >> 3) as usize)
    }

    fn get_bg_tile(&self, tile_index: u8) -> Tile {
        self.get_tile(tile_index, ((self.ppu_ctrl & 0x10) >> 4) as usize)
    }

    fn get_tile(&self, tile_index: u8, pattern_table_num: usize) -> Tile {
        let pattern_table_base : usize = 0x1000 * pattern_table_num;
        let tile_start = pattern_table_base + (tile_index as usize * 16);
        let mut memcopy = [0u8; 16];
        memcopy.copy_from_slice(&self.vram[tile_start..tile_start+16]);
        Tile::from_memory(memcopy)
    }

    fn get_palette(&self, palette_index: u8) -> Palette {
        let palette_mem_loc : usize = 0x3f00 + (palette_index as usize)*4;
        let mut palette_data = [0u8; 4];
        palette_data.copy_from_slice(&self.vram[palette_mem_loc..palette_mem_loc+4]);

        Palette::new(palette_data)
    }

    fn pixel_range_for_line(x: u8) -> core::ops::Range<usize> {
        let x_usize = x as usize;
        let range_width = 4*256; /* 4 bytes per pixel, 256 pixels per line */
        (x_usize*range_width)..(x_usize+1)*range_width
    }

    fn print_nametable(&self) {
        let nametable_size = 0x400;
        let nametable_base = 0x2400 + ((self.ppu_ctrl as usize) & 0x3) * 0x400;
        self.print_vram_memory(nametable_base, nametable_size);
    }

    pub fn print_vram_memory(&self, base_addr: usize, len: usize) {
        let mut mem = Vec::new();
        mem.extend_from_slice(&self.vram[base_addr..(base_addr+len)]);
        let mut output = String::new();
        output.push_str(format!("VRAM Memory [0x{:x}..0x{:x}] : \n", base_addr, base_addr+len).as_str());
        for i in 0..len {
            output.push_str(format!("[0x{:x}]: 0x{:x}\n", base_addr+i, mem[i]).as_str());
        }
        println!("{}", output);
    }

    pub fn vram_address_mirror(addr: usize) -> usize {
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

        result
    }

    fn handle_update(&mut self) {
        loop {
            let update = self.update_receiver.try_recv();
            if update.is_err() {
                break;
            }
            match update.unwrap() {
                CpuToPpuMessage::Memory(addr, data) => {
                    self.vram[PPUState::vram_address_mirror(addr)] = data
                },
                CpuToPpuMessage::Oam(new_oam_data) => {
                    self.oam.copy_from_slice(&new_oam_data)
                }
                CpuToPpuMessage::PpuCtrl(value) => {
                    self.ppu_ctrl = value;
                    self.scroll.nametable = value & 0x3;
                }
                CpuToPpuMessage::PpuMask(value) => {
                    self.ppu_mask = value;
                },
                CpuToPpuMessage::ScrollX(coarse_x, fine_x) => {
                    self.scroll.coarse_x = coarse_x;
                    self.scroll.fine_x = fine_x;
                },
                CpuToPpuMessage::ScrollY(coarse_y, fine_y) => {
                    self.scroll.coarse_y = coarse_y;
                    self.scroll.fine_y = fine_y;
                }
            }
        }
    }

    fn send_update(&self, update: PpuToCpuMessage) {
        self.ppu_update_sender.send(update).expect("");
    }

    fn send_status_update(&self) {
        self.send_update(PpuStatus(self.ppu_status))
    }
}

#[derive(Debug, Clone, Copy)]
struct SpriteInfo
{
    /* NB: this is one less than the top of the sprite! you'll have to add 1 whenever you use it (see get_y) */
    y: u8,
    tile_index: u8,
    attrs: u8,
    x: u8,
    sprite_index: u8,
}

impl SpriteInfo {
    fn in_scanline(&self, scanline: u8, ppu: &PPUState) -> bool {
            self.get_y() <= scanline && scanline - self.get_y() < 8 /* TODO ppu.sprite_size() */
    }

    fn get_y(&self) -> u8 {
        self.y+1
    }

    fn at_x_position(&self, x: u8) -> bool {
        self.x <= x && x - self.x < 8
    }
    fn get_brightness(&self, ppu: &PPUState, x: u8, y: u8) -> u8 {
        self.get_brightness_localized(ppu, x-self.x, y-self.get_y())
    }

    fn get_brightness_localized(&self, ppu: &PPUState, x: u8, y: u8) -> u8 {
        let tile = ppu.get_sprite_tile(self.tile_index); /* TODO is this right? */
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

    fn color_from_brightness(&self, ppu: &PPUState, brightness: u8) -> [u8; 4] {
        ppu.get_palette(self.attrs & 0x3).brightness_to_pixels(brightness)
    }

    fn get_palette(&self, ppu: &PPUState) -> Palette {
        ppu.get_palette((self.attrs & 0x3) + 4)
    }

    /* write this sprite as a byte array into memory */
    fn copy_to_mem(&self, dst_slice: &mut [u8]) {
        dst_slice[0] = self.y;
        dst_slice[1] = self.tile_index;
        dst_slice[2] = self.attrs;
        dst_slice[3] = self.x;
    }

    pub fn is_foreground(&self) -> bool {
        self.attrs & 0x20 == 0
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

fn set_bit_on(flags: u8, bit: u8) -> u8 {
    flags | (1 << bit)
}

fn set_bit_off(flags: u8, bit: u8) -> u8 {
    flags & !(1 << bit)
}