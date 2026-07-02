/// Bank-switched memory array used by cartridge mappers.
///
/// Maps a CPU or PPU address space onto a flat `data` buffer (ROM or RAM)
/// through configurable bank windows. Each bank is `1 << bank_size_log` bytes.
///
/// The `base_address` is the start of the mapped range in the CPU/PPU address
/// space (e.g. `0x8000` for PRG ROM). Reads and writes below `base_address`
/// are no-ops. The `banks` vector holds one entry per bank, controlling which
/// region of `data` that bank maps to.
///
/// Banks are laid out sequentially from `base_address`:
///   bank 0 → [base_address, base_address + bank_size)
///   bank 1 → [base_address + bank_size, base_address + bank_size*2)
///   ...
///
/// Address mapping formula: `banks[address >> bank_size_log] | (address & bank_size_mask)`
pub struct BankArray {
    bank_size_log: usize,
    bank_size: usize,
    bank_size_mask: usize,
    banks: Vec<usize>,
    base_address: usize,
    data: Vec<u8>,
}

impl BankArray {
    /// Creates a zero-filled RAM-backed `BankArray`.
    ///
    /// `data_size` is log2 of the total RAM size in bytes.
    /// `num_banks` is the number of bank slots in the mapping table.
    pub fn new_ram(base_address: u16, bank_size_log:usize, data_size: usize, num_banks: usize) -> BankArray {
        BankArray::new(base_address, bank_size_log, vec![0; 1 << data_size], num_banks)
    }

    /// Creates a new `BankArray` backed by `data`.
    ///
    /// `bank_size_log` is log2 of the bank size (e.g. 14 for 16 KB banks).
    /// `base_address` is where the mapped range starts in the address space.
    /// `num_banks` is the number of bank slots in the mapping table.
    ///
    /// `data.len()` must be a multiple of the bank size. All banks start
    /// mapped to bank 0.
    pub fn new(base_address: u16, bank_size_log:usize, data: Vec<u8>, num_banks: usize) -> BankArray {
        /* if backing data is empty, that means we need to provide RAM */
        let base_address = base_address as usize;
        let bank_size = 1 << bank_size_log;
        assert_eq!(data.len() % bank_size, 0);

        let banks = vec![0; num_banks];

        BankArray {
            base_address,
            bank_size_log,
            bank_size,
            bank_size_mask: bank_size - 1,
            banks,
            data,
        }
    }

    /// Assigns bank slot `bank` to data bank `value`.
    ///
    /// `bank` is the absolute bank index (0-based from start of address space).
    /// `value` selects which bank-sized chunk of `data` to map there.
    ///
    /// Panics if `bank` is out of range.
    pub fn set_bank(&mut self, bank: u8, value: u8) {
        assert!((bank as usize) < self.banks.len());

        self.banks[bank as usize] = value as usize * self.bank_size;
    }

    /// Assigns bank slot `bank` to a bank counted from the end of `data`.
    ///
    /// `value` must be negative: -1 maps to the last bank, -2 to the
    /// second-to-last, etc. Commonly used to pin the upper bank (e.g.
    /// UxROM's fixed last bank).
    ///
    /// Panics if `value` is non-negative or would go out of range.
    pub fn set_bank_from_end(&mut self, bank: u8, value: i8) {
        assert!(value < 0 && -value as usize <= self.banks.len());

        self.banks[bank as usize] = self.data.len() - self.bank_size * (-value as usize)
    }

    /// Reads a single byte from the mapped address space.
    ///
    /// Returns 0 for addresses below `base_address`.
    pub fn read(&self, address: u16) -> u8 {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            self.data[self.map_address(usize_addr - self.base_address)]
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

    /// Changes the size of the banks, reinitializing all banks to bank 0.
    ///
    /// `bank_size_log` is log2 of the new bank size. It must evenly divide
    /// the data size. `num_banks` is the number of bank slots after resizing.
    ///
    /// If `bank_size_log` equals the current bank size, this is a no-op.
    pub fn change_bank_size(&mut self, bank_size_log: usize, num_banks: usize) {
        if(bank_size_log != self.bank_size_log) {
            let bank_size = 1 << bank_size_log;
            /* the data must be a multiple of the bank size */
            assert_eq!(self.data.len() % bank_size, 0);
            self.bank_size_log = bank_size_log;
            self.bank_size = bank_size;
            self.bank_size_mask = self.bank_size - 1;

            self.banks.clear();
            for i in 0..num_banks {
                self.banks.push(0);
            };
        }
    }
}
