use clap::Parser;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::process::exit;
use std::sync::{Arc, Mutex, RwLock};

mod cpu;

mod rom;
use crate::key_event_handler::KeyEventHandler;
use crate::simulator::program_state::ProgramState;
use rom::Rom;

mod apu;
mod config;
mod key_event_handler;
mod mapper;
mod ppu;
mod processor;
mod simulator;
mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let args = CommandLineArgs::parse();

    let rom = Rom::parse_file(args.rom)?;

    let keys = Arc::new(Mutex::new(HashSet::new()));

    let program_state = Arc::new(RwLock::new(ProgramState::simulate_async(
        &rom,
        &args.savefile,
        keys.clone(),
    )));

    let key_event_handler =
        KeyEventHandler::new(keys, program_state.read().unwrap().write_buffer.clone());

    let handler_clone = program_state.clone();
    let savefile_clone = args.savefile.clone();
    ctrlc::set_handler(move || {
        cleanup_and_save(&handler_clone, &savefile_clone);
        exit(0);
    })
    .expect("Should not error due to being only signal handler");

    match window::initialize_ui(program_state.clone(), key_event_handler, args.savefile.clone()) {
        Ok(()) => Ok(()),
        Err(event_loop_error) => Err(event_loop_error.into()),
    }
}

fn cleanup_and_save(program_state: &Arc<RwLock<ProgramState>>, savefile: &Option<String>) {
    let save_data = program_state.write().unwrap().cleanup();
    if let (Some(path), Some(data)) = (savefile, save_data) {
        match fs::write(path, data) {
            Ok(_) => {}
            Err(x) => println!("Failed to write to save file {path}: {x}"),
        }
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
