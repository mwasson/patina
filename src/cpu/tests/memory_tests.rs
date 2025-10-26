use crate::cpu::tests::{memory_for_testing, NoOpMemoryListener};
use crate::cpu::{tests, CoreMemory, MemoryListener};
use crate::rom::Rom;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_memory() {
    let mut memory = tests::memory_for_testing();

    /* TODO this isn't what open bus should actually do */
    assert_eq!(memory.open_bus(), 0);

    /* writing to the FDS ports has no effect; shouldn't try to go to the test mapper,
     * which will just return 0 on read
     */
    memory.write(0x4030, 0xff);
    assert_eq!(memory.read(0x4030), 0);

    /* test address mapping */
    /* PPU registers repeated up to 0x3fff */
    assert_eq!(memory.map_address(0x30ba), 0x2002);
}

#[test]
#[should_panic(expected = "Attempting to register a second memory listener at address 0x4000")]
fn test_only_single_listener_per_address() {
    let mut memory = memory_for_testing();
    let listener_1 = NoOpMemoryListener::new(0x4000);
    let listener_2 = NoOpMemoryListener::new(0x4000);
    memory.register_listener(Rc::new(RefCell::new(listener_1)));
    memory.register_listener(Rc::new(RefCell::new(listener_2)));
}

#[test]
fn test_memory_listener() {
    let mut memory = memory_for_testing();

    /* test write */
    let test_memory_listener = Rc::new(RefCell::new(TestMemoryListener { val: 0x01 }));
    memory.register_listener(test_memory_listener.clone());
    memory.write(0x2000, 0x32);
    assert_eq!(test_memory_listener.borrow().val, 0x32);

    /* test read */
    test_memory_listener.borrow_mut().val = 0x64;
    assert_eq!(memory.read(0x2000), 0x64);
}

#[test]
fn test_copy_from_slice() {
    let mut memory = memory_for_testing();

    for i in 0x100..0x200 {
        memory.write(i, (i % 0x100) as u8);
    }

    let mut destination: [u8; 0x100] = [0; 0x100];
    memory.copy_slice(0x0100, 0x0100, &mut destination);

    for i in 0x00..=0xff {
        assert_eq!(destination[i as usize], i);
    }

    /* test we can do this in the mapper, too */
    for i in 0x8000..0x8100 {
        memory.write(i, (i % 0x100) as u8 - 5);
    }
    memory.copy_slice(0x8000, 0x0100, &mut destination);
    for i in 0x00..=0xff {
        assert_eq!(destination[i as usize], i - 5);
    }
}

#[test]
fn test_memory_from_rom() {
    let rom = basic_test_rom();
    let memory = CoreMemory::new(&rom);
    /* read PRG data */
    assert_eq!(memory.read(0xffff), 0x12);
    assert_eq!(memory.read(0xc000), 0x12);
    assert_eq!(memory.read(0x8000), 0x12);
}

#[test]
#[should_panic]
fn test_memory_from_rom_no_prg_ram() {
    let rom = basic_test_rom();
    let memory = CoreMemory::new(&rom);
    /* attempt to read PRG-RAM */
    memory.read(0x7fff);
}

fn basic_test_rom() -> Rom {
    /* simple test for basic NROM case */
    let mut prg_data = Vec::new();
    for _i in 0..(1 << 15) {
        prg_data.push(0x12);
    }
    let mut chr_data = Vec::new();
    for _i in 0..(1 << 13) {
        chr_data.push(0x34);
    }
    Rom {
        prg_data,
        chr_data,
        byte_6_flags: 0, /* NROM */
        byte_7_flags: 0,
        _trainer: vec![],
        _prg_ram: vec![],
        _tv_system: 0,
    }
}

#[test]
#[should_panic(expected = "(read) Special address 0x2000 doesn't have a registered listener")]
fn read_special_address_without_listener_panics() {
    memory_for_testing().read(0x2000);
}

#[test]
#[should_panic(expected = "(write) Special address 0x2000 doesn't have a registered listener")]
fn write_special_address_without_listener_panics() {
    memory_for_testing().write(0x2000, 0xff);
}

#[test]
#[should_panic(expected = "read16 not supported for special addresses")]
fn read16_special_address_panics() {
    memory_for_testing().read16(0x2000);
}

struct TestMemoryListener {
    val: u8,
}

impl TestMemoryListener {}

impl MemoryListener for TestMemoryListener {
    fn get_addresses(&self) -> Vec<u16> {
        vec![0x2000]
    }

    fn read(&mut self, _memory: &CoreMemory, _address: u16) -> u8 {
        self.val
    }

    fn write(&mut self, _memory: &CoreMemory, _address: u16, value: u8) {
        self.val = value;
    }
}
