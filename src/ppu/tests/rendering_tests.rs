use super::mock_mapper::{make_ppu, make_ppu_with_buffer};
use crate::cpu::tests::test_mapper::TestMapper;
use crate::cpu::{CoreMemory, CPU};
use crate::ppu::palette::Palette;
use crate::ppu::NametableMirroring;

// tick counts derived from: scanline * 341 + dot
const TICKS_TO_VBLANK: usize = 82183; // scanline 241 dot 1 (end_of_screen_render)
const TICKS_TO_PRERENDER_FLAG_CLEAR: usize = 89003; // scanline 261 dot 1
const TICKS_PER_FRAME: usize = 341 * 262;

fn make_test_cpu() -> Box<CPU> {
    CPU::new(Box::new(CoreMemory::new_from_mapper(Box::new(
        TestMapper::new(),
    ))))
}

// ── end_of_screen_render ─────────────────────────────────────────────────────

#[test]
fn end_of_screen_render_sets_vblank_flag() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();
    ppu_rc.borrow_mut().ppu_status = 0;
    ppu_rc.borrow_mut().end_of_screen_render(&mut cpu);
    assert_ne!(ppu_rc.borrow().ppu_status & 0x80, 0);
}

#[test]
fn end_of_screen_render_triggers_nmi_when_ctrl_bit7_set() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();
    ppu_rc.borrow_mut().ppu_ctrl = 1 << 7;
    ppu_rc.borrow_mut().end_of_screen_render(&mut cpu);
    assert!(cpu.nmi_set());
}

#[test]
fn end_of_screen_render_does_not_trigger_nmi_when_ctrl_bit7_clear() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();
    ppu_rc.borrow_mut().ppu_ctrl = 0;
    ppu_rc.borrow_mut().end_of_screen_render(&mut cpu);
    assert!(!cpu.nmi_set());
}

// ── prerender scanline ────────────────────────────────────────────────────────

#[test]
fn prerender_scanline_clears_vblank_overflow_and_sprite0_flags() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();
    // ppu_mask=0 keeps rendering_on=false, so render_pixel is never called and
    // tile/palette caches don't need to be pre-populated.
    ppu_rc.borrow_mut().ppu_status = 0xFF;
    for _ in 0..TICKS_TO_PRERENDER_FLAG_CLEAR {
        ppu_rc.borrow_mut().tick(&mut cpu);
    }
    let status = ppu_rc.borrow().ppu_status;
    assert_eq!(status & (1 << 7), 0, "vblank flag (bit 7) should be cleared");
    assert_eq!(status & (1 << 6), 0, "sprite-0-hit flag (bit 6) should be cleared");
    assert_eq!(status & (1 << 5), 0, "overflow flag (bit 5) should be cleared");
}

// ── pixel integration tests ──────────────────────────────────────────────────
//
// Strategy: run frame 1 with rendering disabled (ppu_mask=0) so the tile and
// palette caches get pre-populated by load_tile/load_palette (which run
// regardless of rendering_on) without render_pixel being called.  Enable
// rendering for frame 2, then tick to vblank so write_buffer is flushed.
//
// Scroll state: t=0 throughout, so v stays 0 across frame 1 and frame 2's
// copy_x/y_bits calls reset v back to 0.  Tile and palette address is
// therefore always nametable 0, tile-index 0, attribute byte 0.
//
// fine_y during scanline S of frame 2 (starting from v=0):
//   fine_y = S % 8  (y_increment fires at dot 256, so S=0 renders with fine_y=0,
//                     S=8 also renders with fine_y=0, etc.)

#[test]
fn background_color_used_when_all_tile_pixels_are_transparent() {
    // CHR = all zeros → pixel_intensity always 0 → fallback to global bg color.
    let (ppu_rc, write_buffer) = make_ppu_with_buffer(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();

    // Frame 1: rendering disabled, warms up tile/palette cache.
    for _ in 0..TICKS_PER_FRAME {
        ppu_rc.borrow_mut().tick(&mut cpu);
    }

    {
        let mut ppu = ppu_rc.borrow_mut();
        ppu.ppu_mask = 0x0A; // enable bg for all columns (bits 1 and 3)
        ppu.write_vram(0x3f00, 0x16); // global bg = distinctive color
    }

    // Frame 2: tick until write_buffer is flushed at vblank.
    for _ in 0..TICKS_TO_VBLANK {
        ppu_rc.borrow_mut().tick(&mut cpu);
    }

    let buf = write_buffer.lock().unwrap();
    let idx = 100 * 1024; // scanline 100, x=0 — safely inside visible area
    assert_eq!(&buf[idx..idx + 4], Palette::hue_lookup(0x16));
}

#[test]
fn tile_palette_color_used_for_nonzero_pixel_intensity() {
    // Tile 0, all 8 rows, low bitplane = 0x80 so pixel at x=0 has intensity 1
    // (swap_bits(0x80) = 0x01; bit 0 of the reversed byte encodes x=0).
    // Palette entry 1 is set to color 0x26; the test verifies that color appears
    // at scanline 16 x=0, where fine_y=0 so row 0 of the tile is sampled.
    let (ppu_rc, write_buffer) = make_ppu_with_buffer(NametableMirroring::Horizontal);
    let mut cpu = make_test_cpu();

    {
        let mut ppu = ppu_rc.borrow_mut();
        // CHR addresses 0x0000–0x0007 are the low bitplane for tile 0 rows 0–7.
        for row in 0u16..8 {
            ppu.write_vram(row as usize, 0x80);
        }
        ppu.write_vram(0x3f01, 0x26); // palette 0 entry 1 = color 0x26
    }

    // Frame 1: rendering disabled.
    for _ in 0..TICKS_PER_FRAME {
        ppu_rc.borrow_mut().tick(&mut cpu);
    }

    ppu_rc.borrow_mut().ppu_mask = 0x0A;

    // Frame 2: tick to vblank.
    for _ in 0..TICKS_TO_VBLANK {
        ppu_rc.borrow_mut().tick(&mut cpu);
    }

    // Scanline 16 has fine_y=0 (it's the 3rd scanline of the second 8-scanline
    // tile row, but fine_y cycles 0..7 per tile row — scanlines 8,16,24,… each
    // start a new cycle with fine_y=0).
    let buf = write_buffer.lock().unwrap();
    let idx = 16 * 1024; // scanline 16, x=0
    assert_eq!(&buf[idx..idx + 4], Palette::hue_lookup(0x26));
}
