pub mod program_state;

mod render_requester;
pub(crate) mod scheduler;

#[cfg(test)]
mod tests;

pub use render_requester::RenderRequester;

pub(crate) enum SimulatorSignal {
    EndSimulation,
}
