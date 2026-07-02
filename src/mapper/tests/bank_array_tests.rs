/*
 * BankArray is the core abstraction for NES cartridge bank switching.
 *
 * NES mappers divide the CPU/PPU address space into fixed-size "banks" that
 * can be remapped to different regions of ROM or RAM at runtime. For example,
 * a 16 KB PRG bank at 0x8000 might point to the first 16 KB of ROM, then be
 * remapped to the second 16 KB to load different game code — all without
 * moving any data.
 *
 * BankArray manages this by keeping a flat data buffer (the ROM/RAM image)
 * and a lookup table (`banks`) that maps each bank slot to an offset in that
 * buffer. The mapper calls set_bank() to change what a slot points to, and
 * BankArray handles translating CPU/PPU addresses to data offsets.
 *
 * The address mapping works by subtracting base_address, then splitting the
 * result into two parts:
 *   - Upper bits select which bank slot to use
 *   - Lower bits select the byte within that bank
 * This is done with a single bitwise OR: banks[addr >> shift] | (addr & mask)
 *
 * The number of bank slots (num_banks) is set at construction and may be
 * smaller than the number of data banks — e.g. UxROM has one switchable
 * 16 KB slot at 0x8000 but many 16 KB banks in its ROM data.
 *
 * These tests use sequential data (data[i] = i & 0xff) so that reading an
 * address reveals exactly which data offset was accessed.
 */

use crate::mapper::bank_array::BankArray;

/* helpers --------------------------------------------------------------- */

/// Expected byte from sequential test data (`data[i] = i & 0xff`),
/// given the bank number and within-bank offset.
fn expected(data_bank: usize, bank_size_log: usize, offset_in_bank: usize) -> u8 {
    ((data_bank * (1 << bank_size_log) | offset_in_bank) & 0xff) as u8
}

/// Build a BankArray with `num_bank_slots` slots backed by `data_banks`
/// banks of sequential data.
fn make_array(bank_size_log: usize, base_address: u16, data_banks: usize, num_bank_slots: usize) -> BankArray {
    let bank_size = 1usize << bank_size_log;
    let data: Vec<u8> = (0..bank_size * data_banks)
        .map(|i| (i & 0xff) as u8)
        .collect();
    BankArray::new(base_address, bank_size_log, data, num_bank_slots)
}

/* construction ---------------------------------------------------------- */

/*
 * When constructed with real ROM data, the contents are preserved.
 * All bank slots start mapped to bank 0.
 */

#[test]
fn test_new_preserves_contents() {
    let data: Vec<u8> = (0..0x8000).map(|i| (i & 0xff) as u8).collect();
    let ba = BankArray::new(0x8000, 14, data, 2);
    /* all banks start at 0, so 0x8000 reads from data[0] */
    assert_eq!(ba.read(0x8000), 0x00);
    assert_eq!(ba.read(0x8001), 0x01);
    assert_eq!(ba.read(0x8100), 0x00); /* 0x100 & 0xff == 0 */
}

/*
 * new_ram() creates a zero-filled buffer. This is how CHR-RAM and mapper
 * RAM are represented — no ROM data, just writable memory.
 */

#[test]
fn test_new_ram_creates_zero_buffer() {
    /* new_ram(base, bank_size_log, data_size_log, num_banks) */
    let ba = BankArray::new_ram(0, 10, 10, 1); /* 1 KB RAM, 1 slot */
    assert_eq!(ba.read(0x000), 0);
    assert_eq!(ba.read(0x3ff), 0);
}

/* base_address boundary ------------------------------------------------- */

/*
 * BankArray only responds to addresses at or above its base_address.
 * For PRG ROM (base 0x8000), anything below that — like RAM at 0x0000-0x07FF
 * or I/O registers — returns 0. This lets the mapper handle other address
 * ranges without BankArray interfering.
 */

#[test]
fn test_read_below_base_returns_zero() {
    let ba = make_array(14, 0x8000, 2, 2);
    assert_eq!(ba.read(0x0000), 0);
    assert_eq!(ba.read(0x7fff), 0);
    assert_eq!(ba.read(0x4000), 0);
}

/* basic bank mapping --------------------------------------------------- */

/*
 * The simplest case: one bank slot mapped to bank 0. Every address in the
 * 0x8000-0xBFFF range reads from the corresponding offset in data.
 *
 * With base_address=0x8000, read() subtracts 0x8000 before mapping, so
 * 0x8000 becomes offset 0, 0x8001 becomes offset 1, etc.
 */

#[test]
fn test_single_bank_read() {
    let mut ba = make_array(14, 0x8000, 1, 1);
    ba.set_bank(0, 0); /* slot 0 → data bank 0 */

    assert_eq!(ba.read(0x8000), expected(0, 14, 0x0000));
    assert_eq!(ba.read(0x8001), expected(0, 14, 0x0001));
    assert_eq!(ba.read(0xbfff), expected(0, 14, 0x3fff));
}

/*
 * Calling set_bank() on a slot changes what data is visible at that slot's
 * address range. This is how mappers like UxROM switch which game code is
 * loaded at 0x8000.
 */

#[test]
fn test_switch_bank_changes_read_mapping() {
    let mut ba = make_array(14, 0x8000, 2, 2);
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 14, 0x0000));

    /* switch slot 0 to data bank 1 */
    ba.set_bank(0, 1);
    /* 0x8001 → relative offset 1, bank 1 → data at 1 * 0x4000 + 1 = 16385 */
    assert_eq!(ba.read(0x8001), expected(1, 14, 0x0001));
}

/*
 * Adjacent bank slots can point to different data independently.
 * A 32 KB PRG window (0x8000-0xFFFF) has two 16 KB slots; each can
 * be remapped separately.
 */

#[test]
fn test_two_banks_independently_mapped() {
    let mut ba = make_array(14, 0x8000, 4, 2);
    ba.set_bank(0, 0); /* slot 0 (0x8000-0xBFFF) → data bank 0 */
    ba.set_bank(1, 2); /* slot 1 (0xC000-0xFFFF) → data bank 2 */

    assert_eq!(ba.read(0x8000), expected(0, 14, 0x0000));
    assert_eq!(ba.read(0xc000), expected(2, 14, 0x0000));
}

/* set_bank_from_end ----------------------------------------------------- */

/*
 * set_bank_from_end() maps a slot to a bank counted from the end of data.
 * A value of -1 means the last bank, -2 the second-to-last, etc.
 *
 * MMC1 uses this to pin the upper bank: set_bank_from_end(1, -1) ensures
 * 0xC000-0xFFFF always reads from the last 16 KB of PRG ROM, regardless
 * of how many banks the cartridge has.
 */

#[test]
fn test_set_bank_from_end_maps_last_bank() {
    /* 4 banks of 8 KB = 32 KB data, 4 slots, base at 0x0000 */
    let mut ba = make_array(13, 0x0000, 4, 4);
    ba.set_bank_from_end(0, -1);

    /* last bank is bank 3 */
    assert_eq!(ba.read(0x0000), expected(3, 13, 0x0000));
}

#[test]
fn test_set_bank_from_end_with_two_slots() {
    /* 2 slots of 16 KB at 0x8000, 2 data banks */
    let mut ba = make_array(14, 0x8000, 2, 2);
    ba.set_bank(0, 0);
    ba.set_bank_from_end(1, -1);

    /* slot 1 → last bank (bank 1) */
    assert_eq!(ba.read(0xc000), expected(1, 14, 0x0000));
    /* slot 0 unchanged */
    assert_eq!(ba.read(0x8000), expected(0, 14, 0x0000));
}

/* change_bank_size ------------------------------------------------------ */

/*
 * change_bank_size() wipes all bank mappings and rebuilds the table with a
 * new bank size. Mappers that support multiple bank modes (like MMC1's 32 KB
 * vs 16 KB PRG switching) call this to resize the windows, then re-map.
 *
 * If the new size equals the current size, it's a no-op — existing mappings
 * are preserved.
 */

#[test]
fn test_change_bank_size_clears_and_remaps() {
    let mut ba = make_array(15, 0x8000, 4, 1); /* start with 32 KB banks, 1 slot */
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 15, 0x0000));

    /* switch to 16 KB banks with 2 slots — all previous mappings are gone */
    ba.change_bank_size(14, 2);
    /* set_bank again to establish mapping */
    ba.set_bank(0, 1);
    ba.set_bank(1, 2);
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x0000));
    assert_eq!(ba.read(0xc000), expected(2, 14, 0x0000));
}

/*
 * Real-world MMC1 scenario: switch from 32 KB mode (one big bank) to
 * 16 KB mode (two independent banks). In 32 KB mode, set_bank(0, 0) covers
 * the whole 0x8000-0xFFFF range. After change_bank_size(14, 2), we need
 * two separate set_bank calls for the upper and lower halves.
 */

#[test]
fn test_change_bank_size_from_32kb_to_16kb() {
    /* 128 KB PRG data, base at 0x8000, 1 slot in 32 KB mode */
    let bank_size = 1usize << 15; /* 32 KB */
    let data: Vec<u8> = (0..(bank_size * 4) as u32)
        .map(|i| (i & 0xff) as u8)
        .collect();
    let mut ba = BankArray::new(0x8000, 15, data, 1);

    /* 32 KB mode: one bank covers 0x8000-0xFFFF */
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 15, 0x0000));

    /* switch to 16 KB mode with 2 slots */
    ba.change_bank_size(14, 2);
    ba.set_bank(0, 1); /* slot 0 (0x8000-0xBFFF) → data bank 1 */
    ba.set_bank(1, 3); /* slot 1 (0xC000-0xFFFF) → data bank 3 */
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x0000));
    assert_eq!(ba.read(0xc000), expected(3, 14, 0x0000));
}

/*
 * change_bank_size is a no-op when the size doesn't change.
 */

#[test]
fn test_change_bank_size_same_size_is_noop() {
    let mut ba = make_array(14, 0x8000, 2, 2);
    ba.set_bank(0, 1);
    ba.set_bank(1, 0);
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x0000));

    /* same bank size — mappings should survive */
    ba.change_bank_size(14, 2);
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x0000));
    assert_eq!(ba.read(0xc000), expected(0, 14, 0x0000));
}

/* read_slice ------------------------------------------------------------ */

/*
 * read_slice() returns a contiguous window of data, useful for fast
 * pattern table reads (PPU fetching tile graphics from CHR memory).
 * Like read(), it returns empty for addresses below base_address.
 */

#[test]
fn test_read_slice_below_base_returns_empty() {
    let ba = make_array(14, 0x8000, 2, 2);
    assert_eq!(ba.read_slice(0x0000, 16), &[] as &[u8]);
}

#[test]
fn test_read_slice_returns_correct_region() {
    /* NOTE: read_slice doesn't subtract base_address, so we use base=0 here.
     * See test_read_slice_below_base_returns_empty for the base>0 case. */
    let data: Vec<u8> = (0..0x4000).map(|i| (i & 0xff) as u8).collect();
    let ba = BankArray::new(0, 14, data, 1);
    let slice = ba.read_slice(0x0000, 4);
    assert_eq!(slice, &[0x00, 0x01, 0x02, 0x03]);
}

#[test]
fn test_read_slice_after_bank_switch() {
    /* NOTE: read_slice doesn't subtract base_address, so we use base=0 here. */
    let mut ba = make_array(14, 0, 2, 2);
    ba.set_bank(0, 1);
    let slice = ba.read_slice(0x0000, 4);
    /* bank 1 offset = 0x4000; sequential data: 0x4000 & 0xff = 0, 0x4001 & 0xff = 1, ... */
    assert_eq!(slice, &[0x00, 0x01, 0x02, 0x03]);
}

/* write (CHR-RAM) ------------------------------------------------------- */

/*
 * write() modifies the underlying data buffer. This is used for CHR-RAM
 * cartridges (like many NROM games) where the PPU can write tile data
 * at runtime. Writes below base_address are silently ignored.
 */

#[test]
fn test_write_below_base_is_noop() {
    let mut ba = make_array(14, 0x8000, 1, 1);
    ba.set_bank(0, 0);
    ba.write(0x0000, 0xFF);
    ba.write(0x7fff, 0xFF);
    /* data should be unaffected — reads still return 0 */
    assert_eq!(ba.read(0x0000), 0x00);
    assert_eq!(ba.read(0x7fff), 0x00);
    /* sanity: the mapped region still works */
    assert_eq!(ba.read(0x8000), 0x00);
}

#[test]
fn test_write_modifies_data_and_read_reflects_it() {
    let mut ba = BankArray::new(0, 13, vec![0; 8192], 1); /* 8 KB CHR-RAM */
    ba.set_bank(0, 0);
    ba.write(0x0100, 0xAB);
    assert_eq!(ba.read(0x0100), 0xAB);
}

/*
 * Writes go through bank mapping, so switching banks and writing again
 * stores to different underlying data. The original bank's data is preserved.
 */

#[test]
fn test_write_to_banked_address() {
    let mut ba = make_array(13, 0x0000, 2, 1);
    ba.set_bank(0, 0);
    ba.write(0x0000, 0x42);
    assert_eq!(ba.read(0x0000), 0x42);

    /* switch slot 0 to data bank 1 and write */
    ba.set_bank(0, 1);
    ba.write(0x0000, 0x99);
    assert_eq!(ba.read(0x0000), 0x99);

    /* original bank's data should still be 0x42 */
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x0000), 0x42);
}

/* 4 KB banks (CHR mode for MMC1) --------------------------------------- */

/*
 * MMC1 can split its 8 KB of CHR memory into two independent 4 KB banks.
 * This lets the PPU use different tile sets for background vs sprites by
 * mapping each 4 KB half to a different region of CHR data.
 */

#[test]
fn test_4kb_chr_banks() {
    /* 8 KB CHR data split into two 4 KB slots */
    let data: Vec<u8> = (0..0x2000).map(|i| (i & 0xff) as u8).collect();
    let mut ba = BankArray::new(0, 12, data, 2);
    ba.set_bank(0, 0); /* slot 0 (0x0000-0x0FFF) → data bank 0 */
    ba.set_bank(1, 1); /* slot 1 (0x1000-0x1FFF) → data bank 1 */

    assert_eq!(ba.read(0x0000), expected(0, 12, 0x0000));
    assert_eq!(ba.read(0x1000), expected(1, 12, 0x0000));

    /* switch first 4 KB to data bank 1 */
    ba.set_bank(0, 1);
    assert_eq!(ba.read(0x0000), expected(1, 12, 0x0000));
}

/* RAM mode -------------------------------------------------------------- */

/*
 * new_ram() creates writable RAM. write/read roundtripping is the primary
 * use case for mapper RAM (e.g. MMC1's 8 KB PRG-RAM at 0x6000-0x7FFF).
 */

#[test]
fn test_ram_mode_read_write_roundtrip() {
    let mut ba = BankArray::new_ram(0, 10, 10, 1); /* 1 KB RAM */
    ba.set_bank(0, 0);
    ba.write(0x000, 0xAA);
    ba.write(0x3FF, 0xBB);
    assert_eq!(ba.read(0x000), 0xAA);
    assert_eq!(ba.read(0x3FF), 0xBB);
}

/* edge cases ------------------------------------------------------------ */

/*
 * The first and last addresses in the mapped range are boundary cases:
 * base_address is the first valid read, and 0xFFFF is the end of the
 * 16-bit address space. Both must map correctly.
 */

#[test]
fn test_read_at_exact_base_address() {
    let mut ba = make_array(14, 0x8000, 1, 1);
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), 0x00);
}

#[test]
fn test_read_last_address_in_range() {
    /* 32 KB at 0x8000, two 16 KB slots */
    let mut ba = make_array(14, 0x8000, 2, 2);
    ba.set_bank(0, 0);
    ba.set_bank(1, 1);
    /* 0xFFFF → subtract base → 0x7FFF
     * → bank_index = 0x7FFF >> 14 = 1, banks[1] = 1 << 14 = 0x4000
     * → offset = 0x7FFF & 0x3FFF = 0x3FFF
     * → mapped = 0x4000 | 0x3FFF = 0x7FFF */
    assert_eq!(ba.read(0xFFFF), expected(1, 14, 0x3fff));
}
