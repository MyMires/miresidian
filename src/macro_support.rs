//! Implementation-only support for generated macros; not a stable node API.

pub use crate::runtime::{block_on, finish_initialization, validate_descriptor};
pub use inventory;

#[cfg(feature = "batch")]
pub use miresidian_batch::{batch_length, batch_outputs};
