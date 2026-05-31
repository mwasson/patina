use crate::apu::APU;
use crate::cpu::tests::test_mapper::TestMapper;
use crate::cpu::{CoreMemory, CPU};
use crate::ppu::{WriteBuffer, WRITE_BUFFER_SIZE, PPU};
use crate::simulator::scheduler::Scheduler;
use crate::simulator::SimulatorSignal;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

fn make_scheduler(mapper: TestMapper) -> (Scheduler, Sender<SimulatorSignal>) {
    let write_buffer: Arc<Mutex<WriteBuffer>> = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));
    let memory = Box::new(CoreMemory::new_from_mapper(Box::new(mapper)));
    let ppu = PPU::new(write_buffer, memory.mapper.clone());
    let apu = APU::new();
    let cpu = CPU::new(memory);
    let (tx, rx) = channel();
    (Scheduler::new(cpu, ppu, apu, rx), tx)
}

#[test]
fn simulate_stops_and_returns_none_without_save_ram() {
    let (mut scheduler, tx) = make_scheduler(TestMapper::new());
    tx.send(SimulatorSignal::EndSimulation).unwrap();
    assert_eq!(scheduler.simulate(), None);
}

#[test]
fn simulate_returns_save_data_on_end_simulation() {
    let expected = vec![1u8, 2, 3, 42];
    let (mut scheduler, tx) = make_scheduler(TestMapper::with_save(expected.clone()));
    tx.send(SimulatorSignal::EndSimulation).unwrap();
    assert_eq!(scheduler.simulate(), Some(expected));
}
