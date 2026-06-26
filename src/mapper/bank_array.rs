/// Bank-switched memory array used by cartridge mappers.
///
/// Maps a CPU or PPU address space onto a flat `data` buffer (ROM or RAM)
/// through configurable bank windows. Each bank is `1 << bank_size_log` bytes.
///
/// The `base_address` is the start of the mapped range in the CPU/PPU address
/// space (e.g. `0x8000` for PRG ROM). Reads and writes below `base_address`
/// are no-ops. The `banks` vector holds one entry per possible bank slot in
/// the full address space; the first `base_address_index` entries are padding
/// for addresses below `base_address` and always map to offset 0.
///
/// Address mapping formula: `banks[address >> bank_size_log] | (address & bank_size_mask)`
pub struct BankArray {
    bank_size_log: usize,
    bank_size: usize,
    bank_size_mask: usize,
    banks: Vec<usize>,
    base_address: usize,
    base_address_index: usize,
    data: Vec<u8>,
}

impl BankArray {
    /// Creates a new `BankArray` backed by `data`.
    ///
    /// `bank_size_log` is log2 of the bank size (e.g. 14 for 16 KB banks).
    /// `base_address` is where the mapped range starts in the address space.
    ///
    /// If `data` is empty, one bank-sized zero buffer is allocated (RAM mode).
    pub fn new(bank_size_log: usize, base_address: u16, data: Vec<u8>) -> Self {
        /* if backing data is empty, that means we need to provide RAM */
        let base_address = base_address as usize;
        let data_non_empty = if data.len() == 0 {
            vec![0; 1 << bank_size_log]
        } else {
            data
        };

        let mut bank_array = BankArray {
            base_address,
            bank_size_log: 0,
            bank_size: 0,
            bank_size_mask: 0,
            base_address_index: 0,
            banks: Vec::new(),
            data: data_non_empty,
        };

        bank_array.change_bank_size(bank_size_log);

        bank_array
    }

    /// Changes the bank size, clearing all existing bank mappings.
    ///
    /// `bank_size_log` is log2 of the new bank size. After this call, the
    /// `banks` vector is reset to only contain padding entries for addresses
    /// below `base_address`; callers must re-establish mappings with `set_bank`.
    pub fn change_bank_size(&mut self, bank_size_log: usize) {
        self.banks.clear();
        self.bank_size_log = bank_size_log;
        self.bank_size = 1 << bank_size_log;
        self.bank_size_mask = self.bank_size - 1;
        self.base_address_index = self.base_address >> self.bank_size_log;
        for _i in 0..self.base_address_index {
            self.banks.push(0);
        }
    }

    /// Maps bank slot `index` to the last bank in `data`.
    ///
    /// Commonly used to pin the upper bank (e.g. UxROM's fixed last bank).
    pub fn set_last_bank(&mut self, index: u8) {
        self.set_bank(index, (self.data.len() >> self.bank_size_log) as u8 - 1);
    }

    /// Assigns bank slot `index` to data bank `bank`.
    ///
    /// `index` is relative to `base_address` (0 = first bank above the base).
    /// `bank` selects which bank-sized chunk of `data` to map there.
    ///
    /// Extends the `banks` vector if `index` equals its current length;
    /// panics if `index` exceeds it.
    pub fn set_bank(&mut self, index: u8, bank: u8) {
        let val = (bank as usize) << self.bank_size_log;
        let index = index as usize + self.base_address_index;
        if index == self.banks.len() {
            self.banks.push(val);
        } else {
            self.banks[index] = val;
        }
    }

    /// Reads a single byte from the mapped address space.
    ///
    /// Returns 0 for addresses below `base_address`.
    pub fn read(&self, address: u16) -> u8 {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            self.data[self.map_address(usize_addr)]
        } else {
            0
        }
    }

    /// Reads a contiguous slice of `size` bytes starting at `address`.
    ///
    /// Returns an empty slice for addresses below `base_address`.
    pub fn read_slice(&self, address: u16, size: usize) -> &[u8] {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            let index = self.map_address(usize_addr);
            &self.data[index..index + size]
        } else {
            &[]
        }
    }

    /// Writes a byte to the mapped address space (for CHR-RAM).
    ///
    /// Silently ignores writes to addresses below `base_address`.
    pub fn write(&mut self, address: u16, value: u8) {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            let index = self.map_address(usize_addr);
            self.data[index] = value;
        }
    }

    /// Translates an address to an index into `data`.
    ///
    /// Uses the upper bits of `address` to select a bank, then ORs the lower
    /// bits (within-bank offset) to produce the final data index.
    fn map_address(&self, address: usize) -> usize {
        self.banks[address >> self.bank_size_log] | (address & self.bank_size_mask)
    }
}
