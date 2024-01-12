#![doc = include_str!("../README.md")]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_variables)
    )
))]
#![forbid(unsafe_code)]
#![warn(clippy::exhaustive_enums, clippy::exhaustive_structs)]
#![allow(
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/12044
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::naive_bytecount,
    clippy::unreadable_literal,
    clippy::wildcard_imports, // TODO
)]

#[cfg(any(feature = "collada", feature = "stl"))]
#[macro_use]
mod error;

#[cfg(any(feature = "collada", feature = "stl"))]
mod utils;

mod common;
pub use common::*;

#[cfg(feature = "collada")]
pub mod collada;
// #[cfg(feature = "obj")]
// pub mod obj;
#[cfg(feature = "stl")]
pub mod stl;

// Not public API. (exposed for benchmarks)
#[doc(hidden)]
#[cfg(any(feature = "collada", feature = "stl"))]
pub mod __private {
    pub use crate::utils::float;
    #[cfg(feature = "collada")]
    pub use crate::utils::int;
}
