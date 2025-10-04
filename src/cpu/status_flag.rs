use crate::cpu::cpu::CPU;

#[derive(Debug)]
pub enum StatusFlag {
    Carry,
    Zero,
    InterruptDisable,
    Decimal,
    /* "No CPU effect; see: the B flag" */
    /* "No CPU effect; always pushed as 1" */
    Overflow,
    Negative,
}

impl StatusFlag {
    pub fn mask(&self) -> u8 {
        match self {
            StatusFlag::Carry => 0,
            StatusFlag::Zero => 1,
            StatusFlag::InterruptDisable => 2,
            StatusFlag::Decimal => 3,
            StatusFlag::Overflow => 6,
            StatusFlag::Negative => 7,
        }
    }

    pub fn is_set(&self, cpu: &CPU) -> bool {
        cpu.status & (1 << self.mask()) != 0
    }

    pub fn as_num(&self, cpu: &CPU) -> u8 {
        (cpu.status & (1 << self.mask())) >> self.mask()
    }

    pub fn update_bool(&self, cpu: &mut CPU, new_val: bool) {
        if new_val {
            cpu.status = cpu.status | (1 << self.mask());
        } else {
            cpu.status = cpu.status & !(1 << self.mask());
        }
    }
}
