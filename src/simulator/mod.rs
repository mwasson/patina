use std::sync::mpsc::Sender;

pub mod program_state;

mod render_requester;
mod scheduler;

pub use render_requester::RenderRequester;

enum SimulatorSignal {
    HandleSave(Sender<Option<Vec<u8>>>),
    EndSimulation,
}
