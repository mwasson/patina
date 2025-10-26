use crate::cpu::{tests, Controller, MemoryListener};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use winit::keyboard::{Key, NamedKey};

#[test]
fn test_controller() {
    let mut controller = Controller::new();
    let key_source = Arc::new(Mutex::new(HashSet::new()));
    controller.set_key_source(key_source.clone());
    {
        let mut unwrapped_key_source = key_source.lock().unwrap();
        unwrapped_key_source.insert(Key::Named(NamedKey::ArrowUp)); // up press
        unwrapped_key_source.insert(Key::Named(NamedKey::Tab)); // select press
        unwrapped_key_source.insert(Key::Character("z".parse().unwrap())); // B press
        unwrapped_key_source.insert(Key::Character("t".parse().unwrap())); // no effect
    }
    controller.record_data();
    /* make sure we got the write output */
    assert_eq!(controller.get_next_byte(), 0); // A off
    assert_eq!(controller.get_next_byte(), 1); // B on
    assert_eq!(controller.get_next_byte(), 1); // select on
    assert_eq!(controller.get_next_byte(), 0); // start off
    assert_eq!(controller.get_next_byte(), 1); // up on
    assert_eq!(controller.get_next_byte(), 0); // down off
    assert_eq!(controller.get_next_byte(), 0); // left off
    assert_eq!(controller.get_next_byte(), 0); // right off
    assert_eq!(controller.get_next_byte(), 1); // will output 1 indefinitely
    assert_eq!(controller.get_next_byte(), 1); // will output 1 indefinitely
    assert_eq!(controller.get_next_byte(), 1); // will output 1 indefinitely

    /* as a memory listener, Controller only listens to one address but will respond
     * to any address--this should be firmed up, to allow for two controllers
     */
    key_source.lock().unwrap().clear(); // clear out keys except start
    key_source
        .lock()
        .unwrap()
        .insert(Key::Named(NamedKey::Enter)); // start press
    let memory = tests::memory_for_testing();
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
    controller.write(&memory, 0x1234, 1);
    assert_eq!(controller.read(&memory, 0x1234), 1); // STILL always returns 1 now
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
    controller.write(&memory, 0x1234, 0); // after this write, we can read!
    assert_eq!(controller.read(&memory, 0x1234), 0); // A off
    assert_eq!(controller.read(&memory, 0x1234), 0); // B off
    assert_eq!(controller.read(&memory, 0x1234), 0); // select off
    assert_eq!(controller.read(&memory, 0x1234), 1); // start on
    assert_eq!(controller.read(&memory, 0x1234), 0); // directions off
    assert_eq!(controller.read(&memory, 0x1234), 0); // directions off
    assert_eq!(controller.read(&memory, 0x1234), 0); // directions off
    assert_eq!(controller.read(&memory, 0x1234), 0); // directions off
    assert_eq!(controller.read(&memory, 0x1234), 1); // always returns 1 now
}
