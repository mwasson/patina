use super::mock_mapper::make_ppu;
use crate::ppu::NametableMirroring;

#[test]
fn horizontal_mirroring_nametables_0_and_2_are_same() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x2000), ppu.vram_address_mirror(0x2800));
    assert_eq!(ppu.vram_address_mirror(0x2400), ppu.vram_address_mirror(0x2C00));
}

#[test]
fn horizontal_mirroring_nametables_0_and_1_are_different() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_ne!(ppu.vram_address_mirror(0x2000), ppu.vram_address_mirror(0x2400));
}

#[test]
fn vertical_mirroring_nametables_0_and_1_are_same() {
    let ppu = make_ppu(NametableMirroring::Vertical);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x2000), ppu.vram_address_mirror(0x2400));
    assert_eq!(ppu.vram_address_mirror(0x2800), ppu.vram_address_mirror(0x2C00));
}

#[test]
fn vertical_mirroring_nametables_0_and_2_are_different() {
    let ppu = make_ppu(NametableMirroring::Vertical);
    let ppu = ppu.borrow();
    assert_ne!(ppu.vram_address_mirror(0x2000), ppu.vram_address_mirror(0x2800));
}

#[test]
fn single_nametable_0_all_map_to_nametable_0() {
    let ppu = make_ppu(NametableMirroring::SingleNametable0);
    let ppu = ppu.borrow();
    let base = ppu.vram_address_mirror(0x2000);
    assert_eq!(ppu.vram_address_mirror(0x2400), base);
    assert_eq!(ppu.vram_address_mirror(0x2800), base);
    assert_eq!(ppu.vram_address_mirror(0x2C00), base);
}

#[test]
fn single_nametable_1_all_map_to_nametable_1() {
    let ppu = make_ppu(NametableMirroring::SingleNametable1);
    let ppu = ppu.borrow();
    let base = ppu.vram_address_mirror(0x2000);
    assert_eq!(ppu.vram_address_mirror(0x2400), base);
    assert_eq!(ppu.vram_address_mirror(0x2800), base);
    assert_eq!(ppu.vram_address_mirror(0x2C00), base);
    assert_eq!(base, 0x2400);
}

#[test]
fn palette_address_3f10_mirrors_to_3f00() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x3f10), 0x3f00);
}

#[test]
fn palette_addresses_wrap_at_3f1f() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x3f20), ppu.vram_address_mirror(0x3f00));
    assert_eq!(ppu.vram_address_mirror(0x3f3f), ppu.vram_address_mirror(0x3f1f));
}

#[test]
fn nametable_range_3000_3eff_mirrors_2000_2eff() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x3000), ppu.vram_address_mirror(0x2000));
    assert_eq!(ppu.vram_address_mirror(0x33ff), ppu.vram_address_mirror(0x23ff));
}

#[test]
fn chr_addresses_below_0x2000_pass_through_unchanged() {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let ppu = ppu.borrow();
    assert_eq!(ppu.vram_address_mirror(0x0000), 0x0000);
    assert_eq!(ppu.vram_address_mirror(0x1000), 0x1000);
    assert_eq!(ppu.vram_address_mirror(0x1fff), 0x1fff);
}
