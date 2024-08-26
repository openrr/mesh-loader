#![doc = include_str!("../README.md")]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_variables)
    )
))]
#![forbid(unsafe_code)]
#![warn(
    // Lints that may help when writing public library.
    missing_debug_implementations,
    // missing_docs, // TODO
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::impl_trait_in_params,
)]
#![allow(
    clippy::inline_always,
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/12044
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
)]

#[cfg(any(feature = "collada", feature = "obj", feature = "stl"))]
#[macro_use]
mod error;

mod utils;

mod loader;
pub use loader::*;
mod common;
pub use common::*;

#[cfg(feature = "collada")]
pub mod collada;
#[cfg(feature = "obj")]
pub mod obj;
#[cfg(feature = "stl")]
pub mod stl;

// Not public API. (exposed for benchmarks)
#[doc(hidden)]
#[cfg(any(feature = "collada", feature = "obj", feature = "stl"))]
pub mod __private {
    pub use crate::utils::float;
    #[cfg(any(feature = "collada", feature = "obj"))]
    pub use crate::utils::int;
}
