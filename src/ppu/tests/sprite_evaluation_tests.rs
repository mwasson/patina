use super::mock_mapper::make_ppu;
use crate::ppu::NametableMirroring;

fn write_sprite(oam: &mut [u8], index: usize, y: u8, tile: u8, attrs: u8, x: u8) {
    let base = index * 4;
    oam[base] = y;
    oam[base + 1] = tile;
    oam[base + 2] = attrs;
    oam[base + 3] = x;
}

// Fills all OAM y-bytes with 0xFF so unwritten slots never land on a test scanline.
fn clear_oam(oam: &mut [u8]) {
    oam.fill(0xFF);
}

#[test]
fn sprite_on_scanline_is_found() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    // y=4 stored → get_y()=5; sprite covers scanlines 5–12
    write_sprite(&mut ppu.oam, 0, 4, 0, 0, 50);
    let sprites = ppu.sprite_evaluation(5);
    assert_eq!(sprites.len(), 1);
}

#[test]
fn sprite_not_on_scanline_is_excluded() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    write_sprite(&mut ppu.oam, 0, 4, 0, 0, 50);
    let sprites = ppu.sprite_evaluation(20);
    assert_eq!(sprites.len(), 0);
}

#[test]
fn sprite_on_last_row_of_8px_sprite_is_found() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    // y=4 → get_y()=5; last row = scanline 12
    write_sprite(&mut ppu.oam, 0, 4, 0, 0, 50);
    assert_eq!(ppu.sprite_evaluation(12).len(), 1);
    assert_eq!(ppu.sprite_evaluation(13).len(), 0);
}

#[test]
fn sprite_evaluation_returns_at_most_8_sprites() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    for i in 0..10 {
        write_sprite(&mut ppu.oam, i, 4, 0, 0, (i * 8) as u8);
    }
    let sprites = ppu.sprite_evaluation(5);
    assert_eq!(sprites.len(), 8);
}

#[test]
fn overflow_flag_set_when_8_or_more_sprites_on_scanline() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    for i in 0..9 {
        write_sprite(&mut ppu.oam, i, 4, 0, 0, (i * 8) as u8);
    }
    ppu.ppu_status = 0;
    ppu.sprite_evaluation(5);
    assert_ne!(ppu.ppu_status & (1 << 1), 0);
}

#[test]
fn overflow_flag_not_set_for_fewer_than_8_sprites() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    for i in 0..7 {
        write_sprite(&mut ppu.oam, i, 4, 0, 0, (i * 8) as u8);
    }
    ppu.ppu_status = 0;
    ppu.sprite_evaluation(5);
    assert_eq!(ppu.ppu_status & (1 << 1), 0);
}

#[test]
fn tall_sprites_cover_16_scanlines() {
    let ppu_rc = make_ppu(NametableMirroring::Horizontal);
    let mut ppu = ppu_rc.borrow_mut();
    clear_oam(&mut ppu.oam);
    ppu.tall_sprites = true;
    // y=10 → get_y()=11; 16px sprite covers scanlines 11–26
    write_sprite(&mut ppu.oam, 0, 10, 0, 0, 50);
    assert_eq!(ppu.sprite_evaluation(26).len(), 1);
    assert_eq!(ppu.sprite_evaluation(27).len(), 0);
}
