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

    pub fn set_last_bank(&mut self, index: u8) {
        self.set_bank(index, (self.data.len() >> self.bank_size_log) as u8 - 1);
    }

    pub fn set_bank(&mut self, index: u8, bank: u8) {
        let val = (bank as usize) << self.bank_size_log;
        let index = index as usize + self.base_address_index;
        if index == self.banks.len() {
            self.banks.push(val);
        } else {
            self.banks[index] = val;
        }
    }

    #[cfg_attr(feature = "profiling", inline(never))]
    pub fn read(&self, address: u16) -> u8 {
        let usize_addr = address as usize;
        if usize_addr >= self.base_address {
            self.data[self.map_address(usize_addr)]
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
    #[cfg_attr(feature = "profiling", inline(never))]
    fn map_address(&self, address: usize) -> usize {
        self.banks[address >> self.bank_size_log] | (address & self.bank_size_mask)
    }
}
