//! This library exposes some of the internals of Ludusavi.
//! Most of this code was not originally written with the intention
//! of making it available as a library,
//! so this is currently presented as-is for you to experiment with.
//! In time, this will be refactored and improved,
//! so please understand that the API will be unstable.

// Some code is only used by the binary crate.
#![allow(unused)]

pub mod api;
mod cloud;
pub mod lang;
pub mod metadata;
pub mod path;
pub mod prelude;
pub mod report;
pub mod resource;
pub mod scan;
mod serialization;
mod wrap;

#[cfg(test)]
mod testing;
