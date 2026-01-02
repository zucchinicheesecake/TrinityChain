// Thin re-export module: implementation is in `blockchain/core.rs` to allow
// progressive decomposition of blockchain responsibilities (validation,
// persistence, chain management, state transitions).

pub mod core;
pub use core::*;
