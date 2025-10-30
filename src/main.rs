use clap::Parser;
use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

mod cpu;

mod rom;
use crate::apu::APU;
use crate::cpu::{CoreMemory, CPU};
use crate::key_event_handler::KeyEventHandler;
use crate::ppu::ppu_listener::PPUListener;
use crate::ppu::{PPU, WRITE_BUFFER_SIZE};
use rom::Rom;
use scheduler::RenderRequester;

mod apu;
mod config;
mod key_event_handler;
mod mapper;
mod ppu;
mod processor;
mod scheduler;
mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let args = CommandLineArgs::parse();

    let rom = Rom::parse_file(args.rom)?;

    let write_buffer = Arc::new(Mutex::new([0; WRITE_BUFFER_SIZE]));
    let write_buffer_clone = write_buffer.clone();

    let keys = Arc::new(Mutex::new(HashSet::new()));
    let key_event_handler = KeyEventHandler::new(keys.clone(), write_buffer.clone());

    let render_listener = Arc::new(Mutex::new(RenderRequester::new()));
    let render_listener_clone = render_listener.clone();

    thread::spawn(move || {
        let mut memory = Box::new(CoreMemory::new(&rom));
        let ppu = PPU::new(
            write_buffer_clone,
            memory.mapper.clone(),
            render_listener_clone,
        );

        let apu = APU::new();
        memory.register_listener(apu.clone());

        let ppu_listener = PPUListener::new(ppu.clone());
        memory.register_listener(Rc::new(RefCell::new(ppu_listener)));

        let mut cpu = CPU::new(memory);
        cpu.set_key_source(keys);
        scheduler::simulate(&mut cpu, ppu, apu);
    });

    match window::initialize_ui(write_buffer, key_event_handler, render_listener) {
        Ok(()) => Ok(()),
        Err(event_loop_error) => Err(event_loop_error.into()),
    }
}

#[derive(Parser, Debug)]
#[command(author = "Mike Wasson", version = "0.0.0 unreleased",
    about, long_about = None)]
struct CommandLineArgs {
    /// rom file
    rom: String,

    /// save file for games with battery-backed saves
    #[arg(short, long)]
    savefile: Option<String>,
}
