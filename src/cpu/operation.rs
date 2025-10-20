use crate::cpu::{from_opcode, RealizedInstruction, CPU};

#[derive(Debug)]
pub struct Operation {
    pub realized_instruction: RealizedInstruction, /* TODO should this be a reference? */
    pub byte1: u8,
    pub byte2: u8,
    extra_cycles: u16,
}

impl Operation {
    pub fn apply(&mut self, cpu: &mut CPU) {
        self.extra_cycles += self.realized_instruction.apply(cpu, self.byte1, self.byte2);
    }

    pub fn cycles(&self) -> u16 {
        self.realized_instruction.cycles + self.extra_cycles
    }

    pub fn operation_from_memory(opcode: u8, byte1: u8, byte2: u8) -> Operation {
        Operation {
            realized_instruction: from_opcode(opcode),
            byte1,
            byte2,
            extra_cycles: 0,
        }
    }
}
