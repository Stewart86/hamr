pub mod config;
pub mod plugin;

// Exposed for benchmarks - not part of stable API
#[doc(hidden)]
pub mod search;

pub(crate) mod frecency;
pub(crate) mod index;
pub(crate) mod platform;

mod engine;
mod error;

#[cfg(test)]
mod tests;

pub use engine::{HamrCore, IndexStats};
pub use error::{Error, Result};

pub use hamr_types::*;
