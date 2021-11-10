use std::{ffi::c_void, marker::PhantomData, ops::Deref};

use dart_api_dl_sys::Dart_InitializeApiDL;

use once_cell::sync::OnceCell;
use thiserror::Error;

static INIT_ONCE: OnceCell<Result<DartRuntime, InitializationFailed>> = OnceCell::new();

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

    /// Asserts this thread is inside of an isolate.
    ///
    /// This will not automatically create a new
    /// dart scope, as any native code based on the
    /// dart dl api will normally
    ///
    /// # Safety
    ///
    /// This must only be called if:
    ///
    /// - We are inside of an isolate (the main dart thread is a isolate, too),
    ///   with a valid dart scope (which if native code is called from dart
    ///   should always be the case).
    pub unsafe fn assert_in_isolate<R>(&self, func: impl FnOnce(&InDartIsolate) -> R) -> R {
        let guard = InDartIsolate {
            runtime: *self,
            _phantom: PhantomData,
        };
        func(&guard)
    }
}

/// Guard for using any dart dl api 's which need to be run in some form of dart scope.
///
/// This acts as a interface to access all dart api dl
/// functions which can only be called from inside of the
/// dart runtime.
pub struct InDartIsolate {
    runtime: DartRuntime,
    _phantom: PhantomData<*mut ()>,
}

impl Deref for InDartIsolate {
    type Target = DartRuntime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
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
        assert_not_impl_any!(InDartIsolate: Send, Sync);
        assert_impl_all!(DartRuntime: Send, Sync);
    }
}
