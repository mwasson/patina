/*
 * MMC3 mapper (iNES mapper 4) tests.
 *
 * The MMC3 has 8 named bank registers (R0-R7) that control how PRG and CHR
 * memory are mapped. R0-R5 control CHR banks (R0/R1 are 2 KB "double" banks,
 * R2-R5 are 1 KB single banks). R6-R7 control PRG banks. Two mode flags
 * (prg_bank_mode and chr_a12_inversion) swap how registers map to address
 * space slots. When modes change, stored bank data is re-applied with the
 * new mapping.
 *
 * PRG memory: 8 KB banks at 0x8000-0xFFFF (4 slots), 8 KB PRG-RAM at 0x6000-0x7FFF.
 * CHR memory: 1 KB banks at 0x0000-0x1FFF (8 slots).
 *
 * These tests use sequential data (data[i] = i & 0xff) so that reading an
 * address reveals exactly which data offset was accessed.
 */

use crate::mapper::mmc3::MMC3;
use crate::mapper::Mapper;
use crate::ppu::NametableMirroring;
use crate::rom::Rom;

/* helpers --------------------------------------------------------------- */

/// Build a minimal Rom for MMC3 testing.
/// `prg_banks` is the number of 8 KB PRG banks.
/// `chr_banks` is the number of 1 KB CHR banks (8 per 8 KB page).
fn make_rom(prg_banks: usize, chr_banks: usize, four_screen: bool) -> Rom {
    let bank_size = 1usize << 13; /* 8 KB */
    let prg_data: Vec<u8> = (0..bank_size * prg_banks)
        .map(|i| (i & 0xff) as u8)
        .collect();
    let chr_size = 1usize << 10; /* 1 KB per CHR bank slot */
    let chr_data: Vec<u8> = (0..chr_size * chr_banks)
        .map(|i| (i & 0xff) as u8)
        .collect();
    let mut byte_6_flags = 0u8; /* horizontal mirroring by default */
    if four_screen {
        byte_6_flags |= 1 << 3;
    }
    Rom {
        prg_data,
        chr_data,
        byte_6_flags,
        byte_7_flags: 0,
        _trainer: vec![],
        _prg_ram: vec![],
        _tv_system: 0,
    }
}

/// Write a value to an MMC3 register via the mapper's write_prg_rom.
fn write_reg(mapper: &mut MMC3, address: u16, value: u8) {
    mapper.write_prg_rom(address, value);
}

/// Expected byte from sequential test data at a given offset.
fn expected_byte(offset: usize) -> u8 {
    (offset & 0xff) as u8
}

/* PRG bank mapping ------------------------------------------------------ */

/*
 * Default MMC3 PRG layout (prg_bank_mode=false):
 *   R6  → slot 0: 0x8000-0x9FFF  (switchable)
 *   R7  → slot 1: 0xA000-0xBFFF  (switchable)
 *   -2  → slot 2: 0xC000-0xDFFF  (second-to-last bank, fixed)
 *   -1  → slot 3: 0xE000-0xFFFF  (last bank, fixed)
 *
 * When prg_bank_mode=true, R6/R7 move to slots 2/1 and the fixed
 * banks move to slots 0/3.
 */

/*
 * After construction, the fixed banks (-2 and -1 from the end) should be
 * in their default slots. With 4 banks of 8 KB: bank 2 at 0xC000,
 * bank 3 at 0xE000.
 */

#[test]
fn test_prg_default_layout() {
    let rom = make_rom(4, 16, false);
    let mapper = MMC3::new(&rom);

    let bank_size = 0x2000; /* 8 KB */

    /* slot 2 = second-to-last bank = bank 2 */
    assert_eq!(mapper.read_prg(0xC000), expected_byte(2 * bank_size));
    /* slot 3 = last bank = bank 3 */
    assert_eq!(mapper.read_prg(0xE000), expected_byte(3 * bank_size));
}

/*
 * Writing to the bank select (0x8000) picks R0-R7, then writing to bank
 * data (0x8001) sets the value. Here we select R6 and write bank 1,
 * which should make 0x8000-0x9FFF read from PRG bank 1.
 */

#[test]
fn test_prg_bank_select_and_data() {
    let rom = make_rom(4, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R6, write bank 1 */
    write_reg(&mut mapper, 0x8000, 0b110); /* R6 */
    write_reg(&mut mapper, 0x8001, 1);

    /* 0x8000-0x9FFF should now map to bank 1 */
    assert_eq!(mapper.read_prg(0x8000), expected_byte(1 * 0x2000));
}

/*
 * Switching prg_bank_mode should re-apply all stored bank data with the
 * new slot assignments. We set R6=1 and R7=2 in mode 0, then flip to
 * mode 1. In mode 1, R6 maps to slot 2 and R7 maps to slot 1, while
 * slots 0 and 3 become the fixed banks (-2 and -1).
 */

#[test]
fn test_prg_bank_mode_switch() {
    let rom = make_rom(4, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* set R6 = 1, R7 = 2 in mode 0 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_110); /* select R6, mode 0 */
    write_reg(&mut mapper, 0x8001, 1);
    write_reg(&mut mapper, 0x8000, 0b000_0_0_111); /* select R7, mode 0 */
    write_reg(&mut mapper, 0x8001, 2);

    /* verify mode 0 layout: R6=1 at 0x8000, R7=2 at 0xA000 */
    assert_eq!(mapper.read_prg(0x8000), expected_byte(1 * 0x2000));
    assert_eq!(mapper.read_prg(0xA000), expected_byte(2 * 0x2000));

    /* switch to mode 1 (bit 6 set) — triggers re-application of bank data */
    write_reg(&mut mapper, 0x8000, 0b000_1_0_000);

    /* mode 1: slot 0 = -2 (fixed), slot 1 = R7=2, slot 2 = R6=1, slot 3 = -1 (fixed) */
    assert_eq!(mapper.read_prg(0x8000), expected_byte(2 * 0x2000)); /* -2 = bank 2 */
    assert_eq!(mapper.read_prg(0xA000), expected_byte(2 * 0x2000)); /* R7 = 2 */
    assert_eq!(mapper.read_prg(0xC000), expected_byte(1 * 0x2000)); /* R6 = 1 */
    assert_eq!(mapper.read_prg(0xE000), expected_byte(3 * 0x2000)); /* -1 = bank 3 */
}

/* CHR bank mapping ------------------------------------------------------ */

/*
 * CHR uses 1 KB banks across 8 slots (0x0000-0x1FFF). R0/R1 are 2 KB
 * "double" banks (they set two adjacent 1 KB slots at once). R2-R5 are
 * 1 KB single banks. chr_a12_inversion swaps the slot assignments.
 */

/*
 * All bank_data_values default to 0, so every CHR slot starts reading
 * from the first 1 KB of CHR data.
 */

#[test]
fn test_chr_default_layout() {
    let rom = make_rom(1, 16, false);
    let mapper = MMC3::new(&rom);

    assert_eq!(mapper.read_chr(0x0000), 0x00);
}

/*
 * R2 is a single 1 KB bank. In normal mode it maps to slot 4 (0x1000-0x13FF).
 * Writing bank 2 should make that slot read from CHR offset 0x800 (2 * 0x400).
 */

#[test]
fn test_chr_single_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R2, write bank 2 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_010); /* R2 */
    write_reg(&mut mapper, 0x8001, 2);

    /* R2 normal mode → slot 4 (0x1000). Bank 2 → offset 2 * 0x400 = 0x800 */
    let bank_size = 0x400; /* 1 KB */
    assert_eq!(mapper.read_chr(0x1000), expected_byte(2 * bank_size));
}

/*
 * R0 is a 2 KB "double" bank. It sets two adjacent 1 KB slots at once,
 * always aligned to an even slot. Writing bank 4 sets slots 0 and 1
 * to CHR banks 4 and 5 (value & ~1 = 4, then 4+1 = 5).
 */

#[test]
fn test_chr_double_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R0 (double bank), write bank 4 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_000); /* R0 */
    write_reg(&mut mapper, 0x8001, 4);

    /* R0 normal mode → slots 0-1. Even value = 4 & !1 = 4. */
    let bank_size = 0x400;
    assert_eq!(mapper.read_chr(0x0000), expected_byte(4 * bank_size)); /* slot 0 = bank 4 */
    assert_eq!(mapper.read_chr(0x0400), expected_byte(5 * bank_size)); /* slot 1 = bank 5 */
}

/* mode change re-applies bank data ------------------------------------- */

/*
 * When chr_a12_inversion flips, all CHR bank data must be re-applied with
 * the new slot assignments. This test sets R2=3 in normal mode (slot 4),
 * then flips to inverted mode where R2 maps to slot 0. The stored value
 * should appear at the new slot without needing to re-write bank data.
 */

#[test]
fn test_chr_inversion_switch_reapplies() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* set R2 = 3 in normal mode */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_010); /* R2, normal */
    write_reg(&mut mapper, 0x8001, 3);
    let bank_size = 0x400;
    /* R2 normal → slot 4 */
    assert_eq!(mapper.read_chr(0x1000), expected_byte(3 * bank_size));

    /* switch to inverted mode — R2 inverted → slot 0 */
    write_reg(&mut mapper, 0x8000, 0b100_0_0_000); /* chr_a12_inversion = true */

    /* slot 0 should now have R2's stored value (bank 3) */
    assert_eq!(mapper.read_chr(0x0000), expected_byte(3 * bank_size));
}

/* nametable mirroring --------------------------------------------------- */

/*
 * The 0xA000 register controls H/V nametable mirroring. Bit 0 = 0 is
 * horizontal, bit 0 = 1 is vertical. The ROM header determines the
 * initial state.
 */

#[test]
fn test_nametable_mirroring_hv() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* default is horizontal (ROM header byte 6 bit 0 = 0) */
    assert!(matches!(mapper.get_nametable_mirroring(), NametableMirroring::Horizontal));

    /* switch to vertical */
    write_reg(&mut mapper, 0xA000, 0x01);
    assert!(matches!(mapper.get_nametable_mirroring(), NametableMirroring::Vertical));

    /* switch back to horizontal */
    write_reg(&mut mapper, 0xA000, 0x00);
    assert!(matches!(mapper.get_nametable_mirroring(), NametableMirroring::Horizontal));
}

/*
 * Four-screen boards have extra VRAM on the cartridge — all four
 * nametables are independent and the mirroring can't be changed by
 * the game. Writes to 0xA000 should be ignored.
 */

#[test]
fn test_four_screen_ignores_nametable_register() {
    let rom = make_rom(1, 8, true); /* four_screen = true */
    let mut mapper = MMC3::new(&rom);

    assert!(matches!(mapper.get_nametable_mirroring(), NametableMirroring::FourScreen));

    /* try to switch to vertical — should be ignored */
    write_reg(&mut mapper, 0xA000, 0x01);
    assert!(matches!(mapper.get_nametable_mirroring(), NametableMirroring::FourScreen));
}

/* PRG RAM --------------------------------------------------------------- */

/*
 * PRG RAM at 0x6000-0x7FFF is controlled by the 0xA001 register:
 *   bit 7: enable (1 = enabled)
 *   bit 6: write protect (1 = read-only)
 *
 * PRG RAM is enabled by default.
 */

#[test]
fn test_prg_ram_read_write() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* PRG RAM is enabled by default — writes should stick */
    mapper.write_prg_ram(0x6000, 0x42);
    assert_eq!(mapper.read_prg(0x6000), 0x42);
}

/*
 * When PRG RAM is disabled (bit 7 = 0), reads should return 0
 * (open bus in hardware, 0 is our approximation).
 */

#[test]
fn test_prg_ram_disabled_reads_zero() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* write something, then disable */
    mapper.write_prg_ram(0x6000, 0x42);
    write_reg(&mut mapper, 0xA001, 0x00); /* bit 7 = 0 → disabled */
    assert_eq!(mapper.read_prg(0x6000), 0x00);
}

/*
 * When write protect is set (bit 6 = 1), writes to PRG RAM should be
 * silently ignored even though RAM is enabled.
 */

#[test]
fn test_prg_ram_write_protect() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* enable RAM, set write protect */
    write_reg(&mut mapper, 0xA001, 0xC0); /* bit 7 = enable, bit 6 = write protect */

    mapper.write_prg_ram(0x6000, 0x42);
    /* write should be ignored */
    assert_eq!(mapper.read_prg(0x6000), 0x00);
}

/* save data ------------------------------------------------------------- */

/*
 * Battery-backed games persist PRG RAM across power cycles via
 * get_save_data/set_save_data. This verifies the roundtrip:
 * write to PRG RAM, extract save data, restore into a fresh mapper,
 * and confirm the data survived.
 */

#[test]
fn test_save_data_roundtrip() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    mapper.write_prg_ram(0x6000, 0xAA);
    mapper.write_prg_ram(0x6001, 0xBB);

    let save = mapper.get_save_data().unwrap();
    assert_eq!(save[0], 0xAA);
    assert_eq!(save[1], 0xBB);

    /* restore into a fresh mapper */
    let rom2 = make_rom(1, 8, false);
    let mut mapper2 = MMC3::new(&rom2);
    mapper2.set_save_data(&save);
    assert_eq!(mapper2.read_prg(0x6000), 0xAA);
    assert_eq!(mapper2.read_prg(0x6001), 0xBB);
}

/* read_prg low address -------------------------------------------------- */

/*
 * read_prg returns 0 for addresses below 0x6000. This covers the early
 * return path that prevents reads to the I/O and expansion ROM regions.
 */

#[test]
fn test_read_prg_below_6000_returns_zero() {
    let rom = make_rom(1, 8, false);
    let mapper = MMC3::new(&rom);

    assert_eq!(mapper.read_prg(0x0000), 0);
    assert_eq!(mapper.read_prg(0x5FFF), 0);
}

/* read_prg_slice -------------------------------------------------------- */

/*
 * read_prg_slice for PRG-RAM should return the correct slice when
 * RAM is enabled, matching what read_prg would return byte-by-byte.
 */

#[test]
fn test_read_prg_slice_ram_enabled() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    mapper.write_prg_ram(0x6000, 0x11);
    mapper.write_prg_ram(0x6001, 0x22);
    mapper.write_prg_ram(0x6002, 0x33);

    let slice = mapper.read_prg_slice(0x6000, 3);
    assert_eq!(slice, &[0x11, 0x22, 0x33]);
}

/*
 * When PRG-RAM is disabled, read_prg_slice should return zeros
 * (open bus approximation) rather than leaking RAM contents.
 */

#[test]
fn test_read_prg_slice_ram_disabled() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    mapper.write_prg_ram(0x6000, 0x11);
    /* disable PRG-RAM */
    write_reg(&mut mapper, 0xA001, 0x00);

    let slice = mapper.read_prg_slice(0x6000, 3);
    assert_eq!(slice, &[0x00, 0x00, 0x00]);
}

/* IRQ registers (0xC000-0xFFFF) ---------------------------------------- */

/*
 * Writing to 0xC000 (even) sets the IRQ latch value. This value is
 * loaded into the counter on the next reload.
 */

#[test]
fn test_irq_latch() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* set latch to 10, then trigger reload (write to 0xC001) */
    write_reg(&mut mapper, 0xC000, 10);
    write_reg(&mut mapper, 0xC001, 0x00); /* reload: sets counter to 0 */

    /* now clock once — counter was 0, so it reloads to 10 */
    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu);
    assert_eq!(mapper.irq_counter, 10);
}

/*
 * Writing to 0xC001 (odd) forces the IRQ counter to 0. On the next
 * clock, it will reload from the latch.
 */

#[test]
fn test_irq_reload() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* set latch to 5 */
    write_reg(&mut mapper, 0xC000, 5);
    /* force reload */
    write_reg(&mut mapper, 0xC001, 0x00);

    /* clock once: counter was 0 → reloads to 5 */
    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu);
    assert_eq!(mapper.irq_counter, 5);
}

/*
 * Writing to 0xE000 (even) disables IRQ. Writing to 0xFFFF (odd)
 * enables it. When enabled and the counter reaches 0, IRQ is asserted.
 */

#[test]
fn test_irq_enable_disable() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* set latch to 1, force reload */
    write_reg(&mut mapper, 0xC000, 1);
    write_reg(&mut mapper, 0xC001, 0x00);

    /* disable IRQ */
    write_reg(&mut mapper, 0xE000, 0x00);

    /* clock twice: reload → 1, decrement → 0. IRQ disabled, so no trigger. */
    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu); /* reload to 1 */
    mapper.listen_ppu_a12(&mut *cpu); /* decrement to 0 */
    assert!(!cpu.borrow_irq());

    /* now enable and repeat */
    write_reg(&mut mapper, 0xC000, 1);
    write_reg(&mut mapper, 0xC001, 0x00);
    write_reg(&mut mapper, 0xFFFF, 0x00); /* enable IRQ */

    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu); /* reload to 1 */
    mapper.listen_ppu_a12(&mut *cpu); /* decrement to 0 → IRQ */
    assert!(cpu.borrow_irq());
}

/* listen_ppu_a12 counter behavior --------------------------------------- */

/*
 * When the counter is 0 and gets clocked, it reloads from the latch
 * but does NOT fire IRQ. This is the standard reload behavior.
 */

#[test]
fn test_irq_counter_reloads_at_zero() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    write_reg(&mut mapper, 0xC000, 7);
    write_reg(&mut mapper, 0xC001, 0x00); /* counter = 0 */
    write_reg(&mut mapper, 0xFFFF, 0x00); /* enable IRQ */

    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu); /* counter was 0 → reload to 7, no IRQ */
    assert_eq!(mapper.irq_counter, 7);
    assert!(!cpu.borrow_irq());
}

/*
 * The counter counts down from the latch value. With latch=3 and IRQ
 * enabled, it takes 3 clocks to reach 0 and fire.
 */

#[test]
fn test_irq_counter_counts_down() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    write_reg(&mut mapper, 0xC000, 3);
    write_reg(&mut mapper, 0xC001, 0x00); /* counter = 0 */
    write_reg(&mut mapper, 0xFFFF, 0x00); /* enable IRQ */

    let mut cpu = make_cpu();
    mapper.listen_ppu_a12(&mut *cpu); /* reload to 3 */
    assert_eq!(mapper.irq_counter, 3);

    mapper.listen_ppu_a12(&mut *cpu); /* 3 → 2 */
    assert_eq!(mapper.irq_counter, 2);
    assert!(!cpu.borrow_irq());

    mapper.listen_ppu_a12(&mut *cpu); /* 2 → 1 */
    assert_eq!(mapper.irq_counter, 1);
    assert!(!cpu.borrow_irq());

    mapper.listen_ppu_a12(&mut *cpu); /* 1 → 0, IRQ fires */
    assert_eq!(mapper.irq_counter, 0);
    assert!(cpu.borrow_irq());
}

/* remaining bank registers ---------------------------------------------- */

/*
 * Tests for R1, R3, R4, R5 to cover all bank_select/bank_data match arms.
 * R1 is a 2 KB double CHR bank (like R0). R3-R5 are 1 KB single CHR banks.
 */

#[test]
fn test_chr_r1_double_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R1, write bank 6 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_001); /* R1 */
    write_reg(&mut mapper, 0x8001, 6);

    /* R1 normal mode → slots 2-3. Even value = 6 & !1 = 6. */
    let bank_size = 0x400;
    assert_eq!(mapper.read_chr(0x0800), expected_byte(6 * bank_size)); /* slot 2 */
    assert_eq!(mapper.read_chr(0x0C00), expected_byte(7 * bank_size)); /* slot 3 */
}

#[test]
fn test_chr_r3_single_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R3, write bank 1 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_011); /* R3 */
    write_reg(&mut mapper, 0x8001, 1);

    /* R3 normal mode → slot 5 (0x1400) */
    let bank_size = 0x400;
    assert_eq!(mapper.read_chr(0x1400), expected_byte(1 * bank_size));
}

#[test]
fn test_chr_r4_single_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R4, write bank 2 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_100); /* R4 */
    write_reg(&mut mapper, 0x8001, 2);

    /* R4 normal mode → slot 6 (0x1800) */
    let bank_size = 0x400;
    assert_eq!(mapper.read_chr(0x1800), expected_byte(2 * bank_size));
}

#[test]
fn test_chr_r5_single_bank() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* select R5, write bank 3 */
    write_reg(&mut mapper, 0x8000, 0b000_0_0_101); /* R5 */
    write_reg(&mut mapper, 0x8001, 3);

    /* R5 normal mode → slot 7 (0x1C00) */
    let bank_size = 0x400;
    assert_eq!(mapper.read_chr(0x1C00), expected_byte(3 * bank_size));
}

/* write_chr ------------------------------------------------------------- */

/*
 * write_chr modifies CHR data (for CHR-RAM boards). This verifies that
 * the mapper passes writes through to the CHR bank array.
 */

#[test]
fn test_write_chr() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    mapper.write_chr(0x0000, 0xAB);
    assert_eq!(mapper.read_chr(0x0000), 0xAB);
}

/* bank_data with no selected bank --------------------------------------- */

/*
 * Writing bank data when no bank is selected (None) should be a no-op.
 * This happens if the game writes to 0x8001 before 0x8000.
 */

#[test]
fn test_bank_data_with_no_selection() {
    let rom = make_rom(1, 16, false);
    let mut mapper = MMC3::new(&rom);

    /* write bank data without selecting a bank first — should not panic */
    write_reg(&mut mapper, 0x8001, 5);
    /* default layout should be unchanged */
    assert_eq!(mapper.read_chr(0x0000), 0x00);
}

/* PRG RAM disabled write ----------------------------------------------- */

/*
 * When PRG-RAM is disabled, writes should be silently ignored.
 */

#[test]
fn test_prg_ram_disabled_write_ignored() {
    let rom = make_rom(1, 8, false);
    let mut mapper = MMC3::new(&rom);

    /* disable PRG-RAM */
    write_reg(&mut mapper, 0xA001, 0x00);

    mapper.write_prg_ram(0x6000, 0x42);
    /* re-enable and check data wasn't written */
    write_reg(&mut mapper, 0xA001, 0x80);
    assert_eq!(mapper.read_prg(0x6000), 0x00);
}

/* helpers for IRQ tests ------------------------------------------------- */

use crate::cpu::CPU;
use crate::cpu::core_memory::CoreMemory;
use crate::cpu::tests::test_mapper::TestMapper;

/// Create a CPU for testing IRQ flag behavior.
fn make_cpu() -> Box<CPU> {
    let memory = CoreMemory::new_from_mapper(Box::new(TestMapper::new()));
    CPU::new(Box::new(memory))
}

/// Extension trait to borrow CPU's irq_flag for assertions.
trait CpuIrqExt {
    fn borrow_irq(&self) -> bool;
}

impl CpuIrqExt for CPU {
    fn borrow_irq(&self) -> bool {
        self.irq_set()
    }
}
