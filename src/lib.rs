#![warn(
    missing_debug_implementations,
    rust_2018_idioms,
    single_use_lifetimes,
    unreachable_pub
)]
#![warn(
    clippy::default_trait_access,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    // clippy::wildcard_imports
)]

#[macro_use]
mod utils;
pub(crate) use utils::*;

mod common;
pub use common::*;

#[cfg(feature = "collada")]
pub mod collada;
#[cfg(feature = "stl")]
pub mod stl;
