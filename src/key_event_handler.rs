use crate::config::SCREENSHOT_KEY;
use crate::ppu::{WriteBuffer, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use chrono::Utc;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageResult};
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::Key;
use winit::keyboard::Key::Character;

type PressedKeys = Arc<Mutex<HashSet<Key>>>;

pub struct KeyEventHandler {
    pressed_keys: PressedKeys,
    write_buffer: Arc<Mutex<WriteBuffer>>,
}

impl KeyEventHandler {
    pub fn new(
        pressed_keys: PressedKeys,
        write_buffer: Arc<Mutex<WriteBuffer>>,
    ) -> KeyEventHandler {
        KeyEventHandler {
            pressed_keys,
            write_buffer,
        }
    }

    // TODO document
    // TODO how will this interact with configuration?
    pub fn handle_key_event(&mut self, key_event: &KeyEvent) {
        match key_event.state {
            ElementState::Pressed => {
                self.pressed_keys
                    .lock()
                    .unwrap()
                    .insert(key_event.logical_key.clone());

                match &key_event.logical_key {
                    Character(key) => match key.as_str() {
                        SCREENSHOT_KEY => {
                            self.take_screenshot();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            ElementState::Released => {
                self.pressed_keys
                    .lock()
                    .unwrap()
                    .remove(&key_event.logical_key.clone());
            }
        }
    }

    // TODO check for errors
    // TODO document
    // TODO screenshot handling should probably be in its own type
    fn take_screenshot(&self) {
        self.take_screenshot_with_path(self.screenshot_path());
    }

    // TODO maybe we can find a better place to put this?
    fn screenshot_path(&self) -> String {
        format!("/tmp/patina--{}.png", Utc::now().to_rfc3339())
    }

    fn take_screenshot_with_path(&self, path: String) -> ImageResult<()> {
        let mut screenshot_path = File::create(path)?;
        let encoder = PngEncoder::new(&screenshot_path);
        encoder.write_image(
            self.write_buffer.lock().unwrap().as_ref(),
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
            ExtendedColorType::Rgba8,
        )?;
        screenshot_path.flush();
        Ok(())
    }

    // fn lol(&self, path: String) -> ImageResult<()> {
    //     let mut test_file = File::create(path)?;
    //     let encoder = PngEncoder::new(&test_file);
    //     encoder.
    //         Ok(())
    // }
    //
    // // TODO move to proper place
    // // TODO check for errors
    // // TODO document
    // fn test(ppu: &PPU, test_png: &str) -> ImageResult<()> {
    //     let mut foo : WriteBuffer = [0; WRITE_BUFFER_SIZE];
    //     let decoder = PngDecoder::new(BufReader::new(File::open(test_png)?))?;
    //     decoder.read_image(&mut foo)?;
    //     if !ppu.internal_buffer.eq(&foo) {
    //         /* write lol to file */
    //         // fail test
    //         assert!(false);
    //     }
    //     Ok(())
    // }
}
