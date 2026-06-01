use super::mock_mapper::{make_ppu, MockMapper};
use crate::cpu::{CoreMemory, MemoryListener};
use crate::ppu::ppu_listener::PPUListener;
use crate::ppu::NametableMirroring;

fn make_listener() -> (PPUListener, std::rc::Rc<std::cell::RefCell<crate::ppu::PPU>>, CoreMemory) {
    let ppu = make_ppu(NametableMirroring::Horizontal);
    let listener = PPUListener::new(ppu.clone());
    let memory = CoreMemory::new_from_mapper(Box::new(MockMapper::new(NametableMirroring::Horizontal)));
    (listener, ppu, memory)
}

#[test]
fn ppuaddr_two_write_sequence_sets_v() {
    let (mut listener, ppu, memory) = make_listener();
    listener.write(&memory, 0x2006, 0x21);
    listener.write(&memory, 0x2006, 0x00);
    assert_eq!(ppu.borrow().internal_regs.v, 0x2100);
}

#[test]
fn ppuaddr_first_write_masks_high_byte_to_6_bits() {
    let (mut listener, ppu, memory) = make_listener();
    // 0xFF masked to 0x3F → high byte in t = 0x3F
    listener.write(&memory, 0x2006, 0xFF);
    listener.write(&memory, 0x2006, 0x00);
    assert_eq!(ppu.borrow().internal_regs.v, 0x3f00);
}

#[test]
fn ppuaddr_second_write_copies_t_to_v() {
    let (mut listener, ppu, memory) = make_listener();
    listener.write(&memory, 0x2006, 0x23);
    listener.write(&memory, 0x2006, 0xC0);
    assert_eq!(ppu.borrow().internal_regs.v, 0x23C0);
}

#[test]
fn ppuaddr_sets_w_latch_on_first_write_and_clears_on_second() {
    let (mut listener, ppu, memory) = make_listener();
    assert_eq!(ppu.borrow().internal_regs.w, false);
    listener.write(&memory, 0x2006, 0x20);
    assert_eq!(ppu.borrow().internal_regs.w, true);
    listener.write(&memory, 0x2006, 0x00);
    assert_eq!(ppu.borrow().internal_regs.w, false);
}

#[test]
fn ppudata_write_stores_to_vram_and_increments_v() {
    let (mut listener, ppu, memory) = make_listener();
    listener.write(&memory, 0x2006, 0x20);
    listener.write(&memory, 0x2006, 0x00);
    listener.write(&memory, 0x2007, 0xAB);
    assert_eq!(ppu.borrow().internal_regs.v, 0x2001);
    assert_eq!(ppu.borrow().read_vram(0x2000), 0xAB);
}

#[test]
fn ppudata_write_increments_v_by_32_when_ctrl_bit2_set() {
    let (mut listener, ppu, memory) = make_listener();
    listener.write(&memory, 0x2000, 0x04);
    listener.write(&memory, 0x2006, 0x20);
    listener.write(&memory, 0x2006, 0x00);
    listener.write(&memory, 0x2007, 0xAB);
    assert_eq!(ppu.borrow().internal_regs.v, 0x2020);
}

#[test]
fn ppudata_read_is_buffered_by_one_cycle() {
    let (mut listener, ppu, memory) = make_listener();
    // Write 0x55 to VRAM at 0x2000 directly
    ppu.borrow_mut().write_vram(0x2000, 0x55);
    // Point v at 0x2000
    listener.write(&memory, 0x2006, 0x20);
    listener.write(&memory, 0x2006, 0x00);
    // First read returns stale buffer (0); second read returns 0x55
    let first = listener.read(&memory, 0x2007);
    let second = listener.read(&memory, 0x2007);
    assert_eq!(first, 0x00);
    assert_eq!(second, 0x55);
}

#[test]
fn ppustatus_read_returns_and_clears_vblank_flag() {
    let (mut listener, ppu, memory) = make_listener();
    ppu.borrow_mut().ppu_status = 0x80;
    let status = listener.read(&memory, 0x2002);
    assert_eq!(status & 0x80, 0x80);
    assert_eq!(ppu.borrow().ppu_status & 0x80, 0x00);
}

#[test]
fn ppustatus_read_resets_w_latch() {
    let (mut listener, ppu, memory) = make_listener();
    ppu.borrow_mut().internal_regs.w = true;
    listener.read(&memory, 0x2002);
    assert_eq!(ppu.borrow().internal_regs.w, false);
}

#[test]
fn ppuscroll_first_write_sets_coarse_x_t_and_fine_x() {
    let (mut listener, ppu, memory) = make_listener();
    // 0b01001011: coarse_x = top 5 bits = 9, fine_x = bottom 3 bits = 3
    listener.write(&memory, 0x2005, 0b01001011);
    assert_eq!(ppu.borrow().internal_regs.get_coarse_x_tmp(), 9);
    assert_eq!(ppu.borrow().internal_regs.get_fine_x(), 3);
    assert_eq!(ppu.borrow().internal_regs.w, true);
}

#[test]
fn ppuscroll_second_write_sets_coarse_y_t_and_fine_y_t() {
    let (mut listener, ppu, memory) = make_listener();
    listener.write(&memory, 0x2005, 0x00);
    // 0b10110101: coarse_y = top 5 bits = 22, fine_y = bottom 3 bits = 5
    listener.write(&memory, 0x2005, 0b10110101);
    assert_eq!(ppu.borrow().internal_regs.get_coarse_y_tmp(), 22);
    assert_eq!(ppu.borrow().internal_regs.get_fine_y_tmp(), 5);
    assert_eq!(ppu.borrow().internal_regs.w, false);
}

#[test]
fn oamdma_copies_page_into_oam() {
    let (mut listener, ppu, mut memory) = make_listener();
    // Write sprite data to page 2 (0x0200-0x02FF) in CPU memory
    memory.write(0x0200, 0x10); // y
    memory.write(0x0201, 0x05); // tile
    memory.write(0x0202, 0x00); // attrs
    memory.write(0x0203, 0x40); // x
    // Trigger DMA for page 0x02
    listener.write(&memory, 0x4014, 0x02);
    let oam = &ppu.borrow().oam;
    assert_eq!(oam[0], 0x10);
    assert_eq!(oam[1], 0x05);
    assert_eq!(oam[2], 0x00);
    assert_eq!(oam[3], 0x40);
}
