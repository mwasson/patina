use clap::Parser;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::process::exit;
use std::sync::{Arc, Mutex, RwLock};

mod cpu;

mod rom;
use crate::key_event_handler::KeyEventHandler;
use rom::Rom;
use simulator::program_state::ProgramState;

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
    if let Some(savefile) = args.savefile.clone() {
        let savefile_clone = savefile.clone();
        ctrlc::set_handler(move || {
            run_on_exit(&savefile_clone, &handler_clone.read().unwrap());
            exit(0);
        })
        .expect("Should not error due to being only signal handler");
    }

    let write_buffer = program_state.read().unwrap().write_buffer.clone();
    let render_requester = program_state.read().unwrap().render_requester.clone();

    let result = match window::initialize_ui(write_buffer, key_event_handler, render_requester) {
        Ok(()) => Ok(()),
        Err(event_loop_error) => Err(event_loop_error.into()),
    };

    if let Some(savefile) = args.savefile {
        run_on_exit(&savefile, &program_state.read().unwrap());
    }

    result
}

fn run_on_exit(savefile: &String, program_state: &ProgramState) {
    if let Some(save_data) = program_state.handle_save() {
        match fs::write(savefile, save_data) {
            Ok(_) => {}
            Err(x) => {
                println!("Failed to write to save file {savefile}: {x}");
            }
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
