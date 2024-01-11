#![forbid(unsafe_code)]
#![warn(clippy::exhaustive_enums, clippy::exhaustive_structs)]

#[cfg(any(feature = "collada", feature = "stl"))]
#[macro_use]
mod error;

mod utils;

mod common;
pub use common::*;

#[cfg(feature = "collada")]
pub mod collada;
// #[cfg(feature = "obj")]
// pub mod obj;
#[cfg(feature = "stl")]
pub mod stl;
