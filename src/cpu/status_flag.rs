use crate::cpu::program_state::ProgramState;

#[derive(Debug)]
pub enum StatusFlag
{
	Carry,
	Zero,
	InterruptDisable,
	Decimal,
	/* "No CPU effect; see: the B flag" */
	/* "No CPU effect; always pushed as 1" */
	Overflow,
	Negative
}

impl StatusFlag
{
	pub fn mask(&self) -> u8 {
		match self {
			StatusFlag::Carry => 0,
			StatusFlag::Zero => 1,
			StatusFlag::InterruptDisable => 2,
			StatusFlag::Decimal => 3,
			StatusFlag::Overflow => 6,
			StatusFlag::Negative => 7
		}
	}

	pub fn is_set(&self, state: &ProgramState) -> bool {
		state.status & (1 << self.mask()) != 0
	}

	pub fn update_bool(&self, state: &mut ProgramState, new_val: bool) {
		let new_val_as_number = if new_val { 1 } else { 0 };
		if new_val {
			state.status = state.status | (1 << self.mask());
		} else {
			state.status = state.status & !(1 << self.mask());
		}
	}
}