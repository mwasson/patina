use crate::mapper::{bank_array, SIZE_8_KB};

/**
 * BankArray is a flexible, efficient and general implementation of the bank-switching
 * mechanism seen in many memory managers.
 *
 * The BankArray consists of a collection of "banks" that are windows into a larger array,
 * data. The BankArray has a size, bank_size (specified in terms of its log: it must be a power
 * of two, in bytes), a base_address it accepts values from, and a list of banks, which control
 * memory from base_address in that order. So:
 *
 * bank 0 handles addresses in [base_address, base_address + bank_size)
 * bank 1 handles [base_address + bank_size, base_address + bank_size*2)
 * bank 2 handles [base_address + bank_size*2, base_address + bank_size*3)
 * etc.
 *
 * Each bank maps to a subarray of data of similar size. We similarly assume that data is split
 * into banks, starting at 0, of the given size.
 *
 * The size and the bank array are reconfigurable on the fly using BankArrayBuilder.
 *
 *
 */
pub struct BankArray {
    bank_size_log: usize,
    bank_size: usize,
    bank_size_mask: usize,
    banks: Vec<usize>,
    base_address: usize,
    data: Vec<u8>,
}

impl BankArray {
    pub fn new_ram(base_address: u16, bank_size_log:usize, data_size: usize, num_banks: usize) -> BankArray {
        BankArray::new(base_address, bank_size_log, vec![0; 1 << data_size], num_banks)
    }

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

    pub fn set_bank(&mut self, bank: u8, value: u8) {
        assert!((bank as usize) < self.banks.len());

        self.banks[bank as usize] = value as usize * self.bank_size;
    }

    pub fn set_bank_from_end(&mut self, bank: u8, value: i8) {
        assert!(value < 0 && -value as usize <= self.banks.len());

        self.banks[bank as usize] = self.data.len() - self.bank_size * (-value as usize)
    }

    pub fn read(&self, address: u16) -> u8 {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            self.data[self.map_address(usize_addr - self.base_address)]
        } else {
            0
        }
    }

    pub fn read_slice(&self, address: u16, size: usize) -> &[u8] {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            let index = self.map_address(usize_addr);
            &self.data[index..index + size]
        } else {
            &[]
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            let index = self.map_address(usize_addr);
            self.data[index] = value;
        }
    }
    /* TODO explain */
    fn map_address(&self, address: usize) -> usize {
        self.banks[address >> self.bank_size_log] | (address & self.bank_size_mask)
    }

    /**
     * Changes the size of the banks to a new value, and reinitializes the banks' data, setting
     * them all to the first bank.
     *
     * The bank size is specified by its log, bank_size_log, so it must be a power of two.
     * It must cleanly divide the data size.
     *
     * If the new bank_size_log is the same as the old one, this has no effect. (It does not
     * clear the banks.)
     */
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