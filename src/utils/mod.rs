#![allow(dead_code)] // TODO

#[macro_use]
pub(crate) mod arena;

pub(crate) mod float;
pub(crate) mod fxhash;
pub(crate) mod hex;
pub(crate) mod int;
pub(crate) mod xml;

// HACK: https://github.com/rust-lang/rust/issues/58733
pub(crate) trait Hack {
    type Output;
}
impl<T> Hack for fn() -> T {
    type Output = T;
}
pub(crate) type Never = <fn() -> ! as Hack>::Output;
