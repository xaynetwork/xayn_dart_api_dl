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

//! This modules provides abstractions around `CObject`.
//!
//! The raw `Dart_CObject` does not only have some rust
//! unsafe types but also needs to be handled differently
//! depending on the context.
//!
//! As such we have multiple types:
//!
//! - [`CObject`] type which is read only.
//!   You will either get a reference to it
//!   from an external source or by dereferencing
//!   [`CObject`].
//!
//! - [`CObject`] an instance we created and as
//!   such we need to handle resource cleanup, like
//!   freeing allocated string.

mod owned;
mod reference;
mod rust_values;
mod type_enums;

pub use owned::*;
pub use reference::*;
pub use rust_values::*;
pub use type_enums::*;
