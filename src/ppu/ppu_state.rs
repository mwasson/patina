use crate::cpu::ProgramState;
use crate::rom::Rom;

use bincode;
use bincode::{BorrowDecode, Encode};

const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
const OAM_SIZE : usize = 256;

const PALETTE_SIZE : usize = 256; /* ? TODO */

const SECONDARY_OAM_SIZE : usize = 32;

struct PPUState<'a> {
    vram: [u8; PPU_MEMORY_SIZE],
    oam: [u8; OAM_SIZE],
    palette_ram: [u8; PALETTE_SIZE],
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
            palette_ram: [0; PALETTE_SIZE],
            secondary_oam: [0; SECONDARY_OAM_SIZE],
            /* TODO: a simple link like this is not sufficient; how should it actually work? */
            ppuctrl: program_state.link_memory(0x2000),
            ppustatus: 0,// program_state.link_memory(0x2002),
            oamaddr: program_state.link_memory(0x2003),
            oamdata: program_state.link_memory(0x2004),
            oamdma: program_state.link_memory(0x4014),
        }
    }

    /* TODO: this is just a sketch; not really sure how I want to use 'cycle' yet */
    fn scanline(&mut self, scanline_num:u8, cycle:u16) {
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

    fn determine_pixel_color() {
        /* TODO */
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

    /* write this sprite as a byte array into memory */
    fn copy_to_mem(&self, dst_slice: &mut [u8]) {
        let result = bincode::encode_into_slice(self, dst_slice, bincode::config::standard());

        if result.is_err() {
            panic!("Failed to copy sprite; {0}", result.unwrap_err());
        }
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