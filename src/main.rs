use clap::Parser;
use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex};

mod cpu;
mod rom;

use crate::key_event_handler::KeyEventHandler;
use crate::simulator::program_state::ProgramState;
use rom::Rom;

mod apu;
mod config;
mod key_event_handler;
mod mapper;
mod menu;
mod ppu;
mod processor;
mod simulator;
mod window;

fn main() -> Result<(), Box<dyn Error>> {
    let args = CommandLineArgs::parse();

    let rom = Rom::parse_file(args.rom)?;
    let keys = Arc::new(Mutex::new(HashSet::new()));
    let program_state = ProgramState::simulate_async(&rom, &args.savefile, keys.clone());
    let key_event_handler = KeyEventHandler::new(keys, program_state.write_buffer.clone());

    window::initialize_ui(program_state, key_event_handler, args.savefile)
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
