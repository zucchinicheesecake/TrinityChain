// core.rs now splits responsibilities into submodules for easier maintenance.
pub mod chain;
pub mod state;
pub mod validation;

pub use chain::*;
pub use state::*;
pub use validation::*;
