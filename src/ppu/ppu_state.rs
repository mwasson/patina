use crate::cpu::ProgramState;
use crate::rom::Rom;


const PPU_MEMORY_SIZE : usize = 1 << 14; /* 16kb */
const OAM_SIZE : usize = 256;

const PALETTE_SIZE : usize = 256; /* ? TODO */

const SECONDARY_OAM_SIZE : usize = 32;

struct PPUState<'a> {
    vram: [u8; PPU_MEMORY_SIZE],
    oam: [u8; OAM_SIZE],
    palette_ram: [u8; PALETTE_SIZE],
    secondary_oam: [u8; SECONDARY_OAM_SIZE],
    oamaddr: &'a u8,
    oamdata: &'a u8,
    oamdma: &'a u8,
}

impl PPUState<'_> { /* TODO: how should the lifetime work here...? */
    pub fn from_rom<'a>(rom: &Rom, program_state: &'a ProgramState) -> PPUState<'a> {
        let mut vram: [u8; PPU_MEMORY_SIZE] = [0; PPU_MEMORY_SIZE];
        let oam : [u8; OAM_SIZE] = [0; OAM_SIZE]; /* TODO: link this to CPU memory? */

        /* copy over character data; TODO surely this is not correct even in the no-mapper case*/
        vram[0x0000..rom.chr_ram.len()].copy_from_slice(&rom.chr_ram);

        return PPUState {
            vram,
            oam,
            palette_ram: [0; PALETTE_SIZE],
            secondary_oam: [0; SECONDARY_OAM_SIZE],
            oamaddr: program_state.link_memory(0x2003),
            oamdata: program_state.link_memory(0x2004),
            oamdma: program_state.link_memory(0x4014),
        }
    }

    fn scanline(&mut self, cycle:u16) {
        /* first 64 cycles: clear secondary oam */
        if(cycle <= 64 && cycle % 2 == 0) {
            self.secondary_oam[(cycle/2) as usize] = 0xff;
        /* 65-256: sprite evaluation */
        } else if (cycle <= 256) {
            /* TODO */
        /* 257-320: sprite fetches */
        } else if (cycle <= 320) {

            /* TODO */
        /* 321-340 + 0 (?): background render pipeline initialization */
        } else {
            /* TODO */
        }
    }

    fn get_sprites_on_scanline(&self, scanline_num: u8) -> Vec<u8> {
        let mut i = 0;
        let mut sprites : Vec<u8> = Vec::new();
        let sprite_size = 8; /* TODO */
        while(i < OAM_SIZE/4) {
            let sprite_data = &self.oam[(i*4)..(i*4+4)];
            if(sprite_data[0] <= scanline_num && sprite_data[0] + sprite_size >= scanline_num) {
                sprites.push(i as u8);
            }
            i += 1;
        }

        sprites
    }
}