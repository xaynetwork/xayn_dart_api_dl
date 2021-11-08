use std::{ffi::c_void, marker::PhantomData};

use dart_api_dl_sys::Dart_InitializeApiDL;

use once_cell::sync::OnceCell;
use thiserror::Error;

static INIT_ONCE: OnceCell<Result<DartRuntime, InitializationFailed>> = OnceCell::new();

thread_local!(static THREAD_IN_DART: OnceCell<InDartRuntime> = OnceCell::new());

/// Initializes the dart api dl.
///
/// Calling any other dart binding functions before this fails.
///
/// It's ok to call this method multiple times and or from multiple threads
/// without any additional synchronization.
///
/// # Safety
///
/// The caller must also make sure that the function pointer slots are not longer
/// used after first this call succeeded and then the dart vm stopped.
///
pub unsafe fn initialize_dart_api_dl(
    initialize_api_dl_data: *mut c_void,
) -> Result<DartRuntime, InitializationFailed> {
    INIT_ONCE
        .get_or_init(|| {
            if Dart_InitializeApiDL(initialize_api_dl_data) == 0 {
                Ok(DartRuntime { _priv: () })
            } else {
                Err(InitializationFailed::InitFailed)
            }
        })
        .clone()
}

/// Marker to prove the dart vm started.
///
/// Acts as an interface for accessing various dart api dl calls.
#[derive(Clone, Copy)]
pub struct DartRuntime {
    _priv: (),
}

impl DartRuntime {
    /// If [`initialize_dart_api_dl`] was called before, this will return the initialization result.
    pub fn instance() -> Result<DartRuntime, InitializationFailed> {
        INIT_ONCE
            .get()
            .cloned()
            .unwrap_or(Err(InitializationFailed::InitNotYetCalled))
    }

    /// Asserts this thread is inside of a dart runtime.
    ///
    /// # Safety
    ///
    /// This must only be called if:
    ///
    /// - We are inside of the dart runtime.
    /// - Any code running in the current thread will either:
    ///   - be also inside of the dart runtime
    ///   - or doesn't try to use dar api dl calls (i.e. dart shutdown)
    pub unsafe fn assert_in_runtime() -> InDartRuntime {
        THREAD_IN_DART.with(|tid| {
            tid.get_or_init(|| InDartRuntime {
                _phantom: PhantomData,
            })
            .clone()
        })
    }
}

/// Marker to prove you are inside of the dart runtime.
///
/// This acts as a interface to access all dart api dl
/// functions which can only be called from inside of the
/// dart runtime.
#[derive(Clone, Copy)]
pub struct InDartRuntime {
    _phantom: PhantomData<*mut ()>,
}

impl InDartRuntime {
    /// Returns a [`InDart]
    pub fn instance() -> Option<Self> {
        THREAD_IN_DART.with(|tid| tid.get().cloned())
    }
}

#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum InitializationFailed {
    #[error("initialize_dart_api_dl was not yet called")]
    InitNotYetCalled,
    #[error("initializing dart api dl failed")]
    InitFailed,
}

#[cfg(test)]
mod tests {
    use static_assertions::{assert_impl_all, assert_not_impl_any};

    use super::*;

    #[test]
    fn test_static_constraints() {
        assert_not_impl_any!(InDartRuntime: Send, Sync);
        assert_impl_all!(DartRuntime: Send, Sync);
    }
}
