pub mod program_state;

pub(crate) mod scheduler;

#[cfg(test)]
mod tests;

pub(crate) enum SimulatorSignal {
    EndSimulation,
}
