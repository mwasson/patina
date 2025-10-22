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
            StatusFlag::Carry => 1 << 0,
            StatusFlag::Zero => 1 << 1,
            StatusFlag::InterruptDisable => 1 << 2,
            StatusFlag::Decimal => 1 << 3,
            StatusFlag::Overflow => 1 << 6,
            StatusFlag::Negative => 1 << 7,
        }
    }

    pub fn is_set(&self, cpu: &CPU) -> bool {
        cpu.status & self.mask() != 0
    }

    pub fn as_num(&self, cpu: &CPU) -> u8 {
        if self.is_set(cpu) { 1 } else { 0 }
    }

    #[inline(always)]
    pub fn update_bool(&self, cpu: &mut CPU, new_val: bool) {
        if new_val {
            cpu.status = cpu.status | self.mask();
        } else {
            cpu.status = cpu.status & !self.mask();
        }
    }
}
