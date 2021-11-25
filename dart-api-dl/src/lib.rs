//! More convenient and safer bindings around `dart-api-dl-sys`.
//!
//! This crate provides wrappers for all functions in `dart_api_dl.h`
//! which can reasonably be used without also using deprecated APIs or
//! the embedding API. This means at least currently no API involving
//! a `Dart_Handle` is provided or should be used.
#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    clippy::missing_errors_doc,
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused_qualifications,
    unsafe_op_in_unsafe_fn,
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::must_use_candidate)]

pub mod cobject;
mod lifecycle;
mod panic;
pub mod port;
mod slot;

pub use lifecycle::*;

pub use dart_api_dl_sys::ILLEGAL_PORT;
