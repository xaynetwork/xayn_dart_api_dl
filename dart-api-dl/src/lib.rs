// Copyright 2021 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    unsafe_op_in_unsafe_fn
)]
#![warn(missing_docs, unreachable_pub)]
//TODO remove
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate, clippy::items_after_statements)]
// We use the zero sized `DartRuntime` type as a guard
// so most of it's functions which have `self` don't use self.
#![allow(clippy::unused_self)]

pub mod cobject;
mod lifecycle;
mod panic;
pub mod ports;

pub use lifecycle::*;

pub use dart_api_dl_sys::ILLEGAL_PORT;
