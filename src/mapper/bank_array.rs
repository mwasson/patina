pub struct BankArray {
    pub bank_size: usize,
    banks: Vec<usize>,
    base_address: usize,
    data: Vec<u8>,
}

impl BankArray {
    pub fn new(bank_size: u16, base_address: u16, data: Vec<u8>) -> Self {
        /* bank_size must be a power of two */
        assert_eq!((bank_size - 1) & bank_size, 0);
        /* base_address must be a multiple of bank_size */
        assert_eq!(base_address % bank_size, 0);

        /* if backing data is empty, guarantee this is backed by one back of RAM */
        let data_non_empty = if data.len() == 0 {
            vec![0; bank_size as usize]
        } else {
            data
        };

        BankArray {
            bank_size: bank_size as usize,
            base_address: base_address as usize,
            banks: Vec::new(),
            data: data_non_empty,
        }
    }

    pub fn change_bank_size(&mut self, bank_size: u16) {
        self.banks.clear();
        self.bank_size = bank_size as usize;
    }

    pub fn set_last_bank(&mut self, index: usize) {
        self.set_bank(index, self.data.len() / self.bank_size - 1);
    }

    pub fn set_bank(&mut self, index: usize, bank: usize) {
        if index == self.banks.len() {
            self.banks.push(bank);
        } else {
            self.banks[index] = bank;
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match self.map_address(address) {
            Some(index) => self.data[index],
            None => 0,
        }
    }

    pub fn read_slice(&self, address: u16, size: usize) -> &[u8] {
        match self.map_address(address) {
            Some(index) => &self.data[index..index + size],
            None => &[],
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match self.map_address(address) {
            Some(index) => self.data[index] = value,
            None => {}
        }
    }

    fn map_address(&self, address: u16) -> Option<usize> {
        if let Some(index_src) = self.index_for_address(address) {
            if let Some(&index_dest) = self.banks.get(index_src) {
                let bank_start_dest = self.bank_size * index_dest;
                let bank_start_src = self.address_for_bank(index_src);
                let offset = address as usize - bank_start_src;

                return Some(bank_start_dest + offset);
            }
        }
        None
    }

    fn address_for_bank(&self, bank: usize) -> usize {
        self.base_address + (bank * self.bank_size)
    }

    fn bank_base_address(&self, address: usize) -> usize {
        address & !(self.bank_size - 1)
    }

    fn index_for_address(&self, address: u16) -> Option<usize> {
        let usize_addr = address as usize;
        if usize_addr < self.base_address {
            None
        } else {
            Some((self.bank_base_address(usize_addr) - self.base_address) / self.bank_size)
        }
    }
}
