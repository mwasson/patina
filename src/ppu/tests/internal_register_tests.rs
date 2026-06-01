use crate::ppu::PPUInternalRegisters;

#[test]
fn fine_y_increments_normally() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_fine_y(3);
    regs.y_increment();
    assert_eq!(regs.get_fine_y(), 4);
    assert_eq!(regs.get_coarse_y(), 0);
}

#[test]
fn fine_y_overflow_increments_coarse_y() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_y(5);
    regs.set_fine_y(7);
    regs.y_increment();
    assert_eq!(regs.get_fine_y(), 0);
    assert_eq!(regs.get_coarse_y(), 6);
}

#[test]
fn coarse_y_29_wraps_and_flips_vertical_nametable_bit() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_y(29);
    regs.set_fine_y(7);
    regs.set_nametable(0b00);
    regs.y_increment();
    assert_eq!(regs.get_coarse_y(), 0);
    assert_eq!(regs.get_fine_y(), 0);
    assert_eq!(regs.get_nametable(), 0b10);
}

#[test]
fn coarse_y_29_with_nametable_2_flips_back_to_0() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_y(29);
    regs.set_fine_y(7);
    regs.set_nametable(0b10);
    regs.y_increment();
    assert_eq!(regs.get_coarse_y(), 0);
    assert_eq!(regs.get_nametable(), 0b00);
}

#[test]
fn coarse_y_31_wraps_without_switching_nametable() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_y(31);
    regs.set_fine_y(7);
    regs.set_nametable(0b01);
    regs.y_increment();
    assert_eq!(regs.get_coarse_y(), 0);
    assert_eq!(regs.get_fine_y(), 0);
    assert_eq!(regs.get_nametable(), 0b01);
}

#[test]
fn coarse_x_increment_normal() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_x(10);
    regs.coarse_x_increment();
    assert_eq!(regs.get_coarse_x(), 11);
    assert_eq!(regs.get_nametable(), 0);
}

#[test]
fn coarse_x_increment_wraps_and_flips_horizontal_nametable_bit() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_x(31);
    regs.set_nametable(0b00);
    regs.coarse_x_increment();
    assert_eq!(regs.get_coarse_x(), 0);
    assert_eq!(regs.get_nametable(), 0b01);
}

#[test]
fn coarse_x_increment_wraps_and_flips_back_from_nametable_1() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_x(31);
    regs.set_nametable(0b01);
    regs.coarse_x_increment();
    assert_eq!(regs.get_coarse_x(), 0);
    assert_eq!(regs.get_nametable(), 0b00);
}

#[test]
fn copy_x_bits_copies_coarse_x_and_horizontal_nametable_bit_from_t() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_x_t(20);
    regs.set_nametable_t(0b11); // bit 0 = 1
    regs.set_coarse_x(0);
    regs.set_nametable(0b10); // bit 1 preserved from v
    regs.copy_x_bits();
    assert_eq!(regs.get_coarse_x(), 20);
    assert_eq!(regs.get_nametable(), 0b11); // bit 1 from v, bit 0 from t
}

#[test]
fn copy_x_bits_preserves_vertical_nametable_bit_in_v() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_nametable_t(0b00); // bit 0 = 0
    regs.set_nametable(0b10);   // bit 1 = 1, should be preserved
    regs.copy_x_bits();
    assert_eq!(regs.get_nametable() & 0b10, 0b10);
    assert_eq!(regs.get_nametable() & 0b01, 0b00);
}

#[test]
fn copy_y_bits_copies_coarse_y_fine_y_and_vertical_nametable_bit_from_t() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_coarse_y_t(18);
    regs.set_fine_y_t(6);
    regs.set_nametable_t(0b11); // bit 1 = 1
    regs.set_nametable(0b01);   // bit 0 = 1, should be preserved
    regs.copy_y_bits();
    assert_eq!(regs.get_coarse_y(), 18);
    assert_eq!(regs.get_fine_y(), 6);
    assert_eq!(regs.get_nametable(), 0b11); // bit 1 from t, bit 0 from v
}

#[test]
fn copy_y_bits_preserves_horizontal_nametable_bit_in_v() {
    let mut regs = PPUInternalRegisters::default();
    regs.set_nametable_t(0b00); // bit 1 = 0
    regs.set_nametable(0b01);   // bit 0 = 1, should be preserved
    regs.copy_y_bits();
    assert_eq!(regs.get_nametable() & 0b01, 0b01);
    assert_eq!(regs.get_nametable() & 0b10, 0b00);
}
