use crate::rom::Rom;
use crate::simulator::program_state::ProgramState;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

fn make_test_rom() -> Rom {
    Rom {
        prg_data: vec![0u8; 16384],
        chr_data: vec![0u8; 8192],
        byte_6_flags: 0,
        byte_7_flags: 0,
        _trainer: vec![],
        _prg_ram: vec![],
        _tv_system: 0,
    }
}

#[test]
fn simulate_async_starts_thread_and_cleanup_stops_it() {
    let keys = Arc::new(Mutex::new(HashSet::new()));
    let mut state = ProgramState::simulate_async(&make_test_rom(), &None, keys);
    assert!(state.thread_handle.is_some());
    assert!(state.cleanup().is_none());
    assert!(state.thread_handle.is_none());
}
