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

use std::ffi::c_void;

use dart_api_dl_sys::Dart_InitializeApiDL;

use displaydoc::Display;
use once_cell::sync::OnceCell;
use thiserror::Error;

static INIT_ONCE: OnceCell<Result<DartRuntime, InitializationFailed>> = OnceCell::new();

/// Alias for the void pointer passed to [`Dart_InitializeApiDL`].
pub type InitData = *mut c_void;

/// Initializes the `dart_api_dl.h` based API.
///
/// Calling any other dart binding functions before this fails.
///
/// It's ok to call this method multiple times and/or from multiple threads
/// without any additional synchronization.
///
/// # Errors
///
/// This can produce an [`InitializationFailed::InitFailed`] error if initialization
/// fails. Dart doesn't tell us why initialization failed, but the only likely reason
/// is that the major version associated with `dart_api_dl.h` of the Dart VM doesn't
/// match the major version of the `dart_api_dl.h` we build against.
///
/// # Safety
///
/// The caller must also make sure that the function pointer slots are no longer
/// used after first this call succeeded and then the Dart VM stopped.
///
/// This is a rather leaky unsafe abstraction but we do not really have any
/// control at all over the dart VM stopping, nor reliable "just before stop"
/// callbacks.
///
/// Luckily even after the Dart VM stops all of the functionality exposed here
/// should be rust-safe to call (but might abort the process), though there
/// are no guarantees.
//FIXME: we could have a Dart VM shutdown guard by returning an external typed data
// "with magic" destructor the user has to place in a static variable. But besides
// it being un-ergonomic it's also very confusing/error prone with the current external
// typed data and if blocking finalizers doesn't block the shutdown also doesn't work.
// Maybe with external native pointers (like added in dart 2.15) this will get a bit
// better.
pub unsafe fn initialize_dart_api_dl(
    initialize_api_dl_data: InitData,
) -> Result<DartRuntime, InitializationFailed> {
    INIT_ONCE
        .get_or_init(|| {
            if unsafe { Dart_InitializeApiDL(initialize_api_dl_data) } == 0 {
                Ok(DartRuntime { _priv: () })
            } else {
                Err(InitializationFailed::InitFailed)
            }
        })
        .clone()
}

/// Marker to prove the Dart VM started.
///
/// Acts as an interface for accessing various dart api dl calls.
#[derive(Clone, Copy)]
pub struct DartRuntime {
    _priv: (),
}

impl DartRuntime {
    /// If [`initialize_dart_api_dl`] was called before, this will return the initialization result.
    ///
    /// # Errors
    ///
    /// - If [`initialize_dart_api_dl`] was not yet called.
    pub fn instance() -> Result<Self, InitializationFailed> {
        INIT_ONCE
            .get()
            .cloned()
            .unwrap_or(Err(InitializationFailed::InitNotYetCalled))
    }

    pub(crate) unsafe fn instance_unchecked() -> Self {
        DartRuntime { _priv: () }
    }
}

/// Error representing that initialization failed.
#[derive(Debug, Clone, Error, Display)]
#[non_exhaustive]
pub enum InitializationFailed {
    /// Initialization was not yet done.
    InitNotYetCalled,
    /// Initialization failed.
    InitFailed,
}

/// The slot for given function pointer was not initialized.
///
/// This can happen in two cases:
///
/// 1. The API was not successfully initialized,
///    which is especially bad as reading the slots before initialization
///    can cause unsound behavior due to race conditions.
/// 2. The function is not supported in the API version used by the VM.
#[derive(Debug, Error)]
#[error("uninitialized function slot: {}", _0)]
pub struct UninitializedFunctionSlot(pub(crate) &'static str);

macro_rules! fpslot {
    (@call $slot:ident ( $($pn:expr),* )) => (
        match $slot {
            Some(func) => Ok(func($($pn),*)),
            None => Err($crate::lifecycle::UninitializedFunctionSlot(stringify!($slot))),
        }
    );
}

pub(crate) use fpslot;

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[test]
    fn test_static_constraints() {
        assert_impl_all!(DartRuntime: Send, Sync);
    }
}
