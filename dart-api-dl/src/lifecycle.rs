use std::ffi::c_void;

use dart_api_dl_sys::Dart_InitializeApiDL;

use once_cell::sync::OnceCell;
use thiserror::Error;

static INIT_ONCE: OnceCell<Result<DartRuntime, InitializationFailed>> = OnceCell::new();

pub type InitData = *mut c_void;

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
    initialize_api_dl_data: InitData,
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
    pub fn instance() -> Result<Self, InitializationFailed> {
        INIT_ONCE
            .get()
            .cloned()
            .unwrap_or(Err(InitializationFailed::InitNotYetCalled))
    }

    #[cfg(test)]
    pub(crate) unsafe fn instance_unchecked() -> Self {
        DartRuntime { _priv: () }
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
    use static_assertions::assert_impl_all;

    use super::*;

    #[test]
    fn test_static_constraints() {
        // assert_not_impl_any!(InDartIsolate: Send, Sync);
        assert_impl_all!(DartRuntime: Send, Sync);
    }
}
