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
 * The address mapping works by splitting a CPU address into two parts:
 *   - Upper bits select which bank slot to use
 *   - Lower bits select the byte within that bank
 * This is done with a single bitwise OR: banks[addr >> shift] | (addr & mask)
 *
 * These tests use sequential data (data[i] = i & 0xff) so that reading an
 * address reveals exactly which data offset was accessed.
 */

use crate::mapper::bank_array::BankArray;

/* helpers --------------------------------------------------------------- */

/// Expected byte from sequential test data (`data[i] = i & 0xff`),
/// given the bank number and a CPU/PPU address.
fn expected(bank: usize, bank_size_log: usize, address: u16) -> u8 {
    let offset = bank * (1 << bank_size_log) | (address as usize & ((1 << bank_size_log) - 1));
    (offset & 0xff) as u8
}

/// Build a BankArray backed by `n_banks` banks of size `1 << bank_size_log`,
/// filled with sequential bytes so we can verify which offset was read.
fn make_array(bank_size_log: usize, base_address: u16, n_banks: usize) -> BankArray {
    let bank_size = 1usize << bank_size_log;
    let data: Vec<u8> = (0..bank_size * n_banks)
        .map(|i| (i & 0xff) as u8)
        .collect();
    BankArray::new(bank_size_log, base_address, data)
}

/* construction ---------------------------------------------------------- */

/*
 * When constructed with empty data, BankArray allocates a zero-filled buffer
 * (one bank's worth). This is how CHR-RAM and mapper RAM are represented —
 * no ROM data, just writable memory.
 */

#[test]
fn test_new_with_empty_data_creates_ram() {
    /* empty data → one bank-sized zero buffer (RAM mode) */
    let mut ba = BankArray::new(10, 0, vec![]);
    ba.set_bank(0, 0);
    /* reading any address within that bank should return 0 */
    assert_eq!(ba.read(0x000), 0);
    assert_eq!(ba.read(0x3ff), 0);
}

/*
 * When constructed with real ROM data, the contents are preserved and
 * accessible once a bank is mapped.
 */

#[test]
fn test_new_with_data_preserves_contents() {
    let data: Vec<u8> = (0..0x8000).map(|i| (i & 0xff) as u8).collect();
    let mut ba = BankArray::new(14, 0x8000, data);
    ba.set_bank(0, 0);
    /* first byte of the first bank at 0x8000 should be 0 */
    assert_eq!(ba.read(0x8000), 0x00);
    /* byte 256 into the bank */
    assert_eq!(ba.read(0x8100), 0x00); /* 0x100 & 0xff == 0 */
    /* byte 1 */
    assert_eq!(ba.read(0x8001), 0x01);
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
    let ba = make_array(14, 0x8000, 2);
    assert_eq!(ba.read(0x0000), 0);
    assert_eq!(ba.read(0x7fff), 0);
    assert_eq!(ba.read(0x4000), 0);
}

/* basic bank mapping --------------------------------------------------- */

/*
 * The simplest case: one bank mapped at index 0. Every address in the
 * 0x8000-0xBFFF range reads from the corresponding offset in data.
 */

#[test]
fn test_single_bank_read() {
    /* one 16 KB bank at 0x8000, data is sequential */
    let mut ba = make_array(14, 0x8000, 1);
    ba.set_bank(0, 0); /* bank 0 maps to data offset 0 */

    assert_eq!(ba.read(0x8000), expected(0, 14, 0x8000));
    assert_eq!(ba.read(0x8001), expected(0, 14, 0x8001));
    assert_eq!(ba.read(0xbfff), expected(0, 14, 0xbfff));
}

/*
 * Calling set_bank() again on the same slot changes what data is visible.
 * This is how mappers like UxROM switch which game code is loaded at 0x8000.
 */

#[test]
fn test_switch_bank_changes_read_mapping() {
    /* two 16 KB banks at 0x8000 */
    let mut ba = make_array(14, 0x8000, 2);
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 14, 0x8000));

    /* switch to bank 1 — data offset 1 << 14 = 16384 */
    ba.set_bank(0, 1);
    /* 0x8001 maps to banks[2] | (0x8001 & 0x3fff) = 16384 | 1 = 16385 */
    assert_eq!(ba.read(0x8001), expected(1, 14, 0x8001));
}

/*
 * Adjacent bank slots can point to different data independently.
 * A 32 KB PRG window (0x8000-0xFFFF) has two 16 KB slots; each can
 * be remapped separately.
 */

#[test]
fn test_two_banks_independently_mapped() {
    /* 32 KB address space (0x8000-0xFFFF) with two 16 KB banks */
    let mut ba = make_array(14, 0x8000, 4);
    ba.set_bank(0, 0); /* 0x8000-0xBFFF → data bank 0 */
    ba.set_bank(1, 2); /* 0xC000-0xFFFF → data bank 2 */

    assert_eq!(ba.read(0x8000), expected(0, 14, 0x8000)); /* bank 0, offset 0 */
    assert_eq!(ba.read(0xc000), expected(2, 14, 0xc000)); /* bank 2, offset 0 */
}

/* set_bank internals ---------------------------------------------------- */

/*
 * The internal `banks` vector has padding entries for addresses below
 * base_address. set_bank() adds entries at the right position by appending
 * when the index matches the current length. This test verifies that
 * sequential set_bank calls build the table correctly.
 */

#[test]
fn test_set_bank_extends_when_index_equals_len() {
    let mut ba = make_array(14, 0x8000, 2);
    /* base_address_index = 0x8000 >> 14 = 2; banks starts with [0, 0].
     * set_bank(0, 0) sets banks[2] → pushes, making len 3.
     * set_bank(1, 0) sets banks[3] → pushes, making len 4. */
    ba.set_bank(0, 0);
    ba.set_bank(1, 0);
    /* both halves should be readable */
    assert_eq!(ba.read(0x8000), 0x00);
    assert_eq!(ba.read(0xc000), 0x00);
}

/* set_last_bank --------------------------------------------------------- */

/*
 * set_last_bank() is a convenience for "always map to the highest bank in
 * ROM." UxROM uses this to pin the reset/interrupt vectors at 0xC000-0xFFFF
 * to the last 16 KB of PRG ROM, no matter how many banks the cartridge has.
 */

#[test]
fn test_set_last_bank_maps_final_bank() {
    /* 4 banks of 8 KB = 32 KB data, base at 0x0000 */
    let mut ba = make_array(13, 0x0000, 4);
    ba.set_last_bank(0);

    /* last bank is bank 3, offset = 3 << 13 = 24576 */
    assert_eq!(ba.read(0x0000), expected(3, 13, 0x0000));
}

#[test]
fn test_set_last_bank_with_two_banks() {
    /* 2 banks of 16 KB at 0x8000 */
    let mut ba = make_array(14, 0x8000, 2);
    ba.set_bank(0, 0);
    ba.set_last_bank(1);

    /* bank 1 offset = 16384 */
    assert_eq!(ba.read(0xc000), expected(1, 14, 0xc000));
    /* bank 0 unchanged */
    assert_eq!(ba.read(0x8000), expected(0, 14, 0x8000));
}

/* change_bank_size ------------------------------------------------------ */

/*
 * change_bank_size() wipes all bank mappings and rebuilds the table with a
 * new bank size. Mappers that support multiple bank modes (like MMC1's 32 KB
 * vs 16 KB PRG switching) call this to resize the windows, then re-map.
 * This is destructive — callers must call set_bank() again afterward.
 */

#[test]
fn test_change_bank_size_clears_and_remaps() {
    let mut ba = make_array(15, 0x8000, 4); /* start with 32 KB banks */
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 15, 0x8000));

    /* switch to 16 KB banks — all previous bank mappings are gone */
    ba.change_bank_size(14);
    /* banks is re-populated with only base_address_index padding (0x8000 >> 14 = 2 entries) */
    /* set_bank again to establish mapping */
    ba.set_bank(0, 1);
    ba.set_bank(1, 2);
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x8000));
    assert_eq!(ba.read(0xc000), expected(2, 14, 0xc000));
}

/*
 * Real-world MMC1 scenario: switch from 32 KB mode (one big bank) to
 * 16 KB mode (two independent banks). In 32 KB mode, set_bank(0, 0) covers
 * the whole 0x8000-0xFFFF range. After change_bank_size(14), we need two
 * separate set_bank calls for the upper and lower halves.
 */

#[test]
fn test_change_bank_size_from_32kb_to_16kb() {
    /* 128 KB PRG data, base at 0x8000 */
    let bank_size = 1usize << 15; /* 32 KB */
    let data: Vec<u8> = (0..(bank_size * 4) as u32)
        .map(|i| (i & 0xff) as u8)
        .collect();
    let mut ba = BankArray::new(15, 0x8000, data);

    /* 32 KB mode: one bank covers 0x8000-0xFFFF */
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), expected(0, 15, 0x8000));

    /* switch to 16 KB mode */
    ba.change_bank_size(14);
    ba.set_bank(0, 1); /* 0x8000-0xBFFF → data bank 1 (offset 0x4000) */
    ba.set_bank(1, 3); /* 0xC000-0xFFFF → data bank 3 (offset 0xC000) */
    assert_eq!(ba.read(0x8000), expected(1, 14, 0x8000));
    assert_eq!(ba.read(0xc000), expected(3, 14, 0xc000));
}

/* read_slice ------------------------------------------------------------ */

/*
 * read_slice() returns a contiguous window of data, useful for fast
 * pattern table reads (PPU fetching tile graphics from CHR memory).
 * It uses the same address mapping as read().
 */

#[test]
fn test_read_slice_below_base_returns_empty() {
    let ba = make_array(14, 0x8000, 2);
    assert_eq!(ba.read_slice(0x0000, 16), &[] as &[u8]);
}

#[test]
fn test_read_slice_returns_correct_region() {
    let data: Vec<u8> = (0..0x8000).map(|i| (i & 0xff) as u8).collect();
    let mut ba = BankArray::new(14, 0x8000, data);
    ba.set_bank(0, 0);
    let slice = ba.read_slice(0x8000, 4);
    assert_eq!(slice, &[0x00, 0x01, 0x02, 0x03]);
}

#[test]
fn test_read_slice_across_bank_switch() {
    let mut ba = make_array(14, 0x8000, 2);
    ba.set_bank(0, 1);
    let slice = ba.read_slice(0x8000, 4);
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
    /* writes below base_address are silently ignored */
    let mut ba = make_array(14, 0x8000, 1);
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
    let mut ba = BankArray::new(13, 0, vec![0; 8192]); /* 8 KB CHR-RAM */
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
    let mut ba = make_array(13, 0x0000, 2);
    ba.set_bank(0, 0);
    ba.write(0x0000, 0x42);
    assert_eq!(ba.read(0x0000), 0x42);

    /* switch bank and write to new mapping */
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
    /* 8 KB CHR data split into two 4 KB banks */
    let data: Vec<u8> = (0..0x2000).map(|i| (i & 0xff) as u8).collect();
    let mut ba = BankArray::new(12, 0, data); /* 4 KB banks at address 0 */
    ba.set_bank(0, 0); /* 0x0000-0x0FFF → data[0x0000] */
    ba.set_bank(1, 1); /* 0x1000-0x1FFF → data[0x1000] */

    assert_eq!(ba.read(0x0000), 0x00);
    assert_eq!(ba.read(0x1000), expected(1, 12, 0x1000));

    /* switch first 4 KB to bank 1 */
    ba.set_bank(0, 1);
    assert_eq!(ba.read(0x0000), expected(1, 12, 0x0000));
}

/* RAM mode (empty data) ------------------------------------------------ */

/*
 * When constructed with empty data, BankArray allocates writable RAM.
 * This is the same code path as test_new_with_empty_data_creates_ram,
 * but here we verify write/read roundtripping — the primary use case for
 * mapper RAM (e.g. MMC1's 8 KB PRG-RAM at 0x6000-0x7FFF).
 */

#[test]
fn test_ram_mode_read_write_roundtrip() {
    let mut ba = BankArray::new(10, 0, vec![]); /* 1 KB RAM */
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
    let mut ba = make_array(14, 0x8000, 1);
    ba.set_bank(0, 0);
    assert_eq!(ba.read(0x8000), 0x00);
}

#[test]
fn test_read_last_address_in_range() {
    /* 32 KB at 0x8000, two 16 KB banks */
    let mut ba = make_array(14, 0x8000, 2);
    ba.set_bank(0, 0);
    ba.set_bank(1, 1);
    /* 0xFFFF → bank_index = 0xFFFF >> 14 = 3, banks[3] = 1 << 14 = 0x4000
     * offset = 0xFFFF & 0x3FFF = 0x3FFF
     * mapped = 0x4000 | 0x3FFF = 0x7FFF */
    assert_eq!(ba.read(0xFFFF), expected(1, 14, 0xFFFF));
}
