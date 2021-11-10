use std::{
    ffi::CStr,
    fmt::{self, Debug, Display},
};

use dart_api_dl_sys::{Dart_GetError_DL, Dart_Handle, Dart_IsError_DL};

use crate::{slot::fpslot, InDartIsolate};

//todo not safe, handler validity is not given
pub struct DartHandle<'a> {
    handle: Dart_Handle,
    _scope: &'a InDartIsolate,
}

impl Debug for DartHandle<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DartHandle(<handle>)")
    }
}

impl<'a> DartHandle<'a> {
    /// Create a new handle.
    ///
    /// # Safety
    ///
    /// Must be a valid handle.
    pub unsafe fn from_raw(scope: &'a InDartIsolate, handle: Dart_Handle) -> Self {
        DartHandle {
            handle,
            _scope: scope,
        }
    }

    /// Return the internal raw handle.
    ///
    /// # Safety
    ///
    /// If you somehow invalidate the handle you must make
    /// sure to not use this instance of self again.
    pub unsafe fn raw_handle(&self) -> Dart_Handle {
        self.handle
    }

    pub fn is_error(&self) -> bool {
        // Safe:
        // The handle is valid, hence it's safe
        unsafe { fpslot!(@call Dart_IsError_DL(self.handle)) }
    }

    pub fn into_result(self) -> Result<Self, DartError<'a>> {
        if self.is_error() {
            Err(DartError(self))
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug)]
pub struct DartError<'a>(DartHandle<'a>);

impl Display for DartError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DartError: {}", self.error_string())
    }
}

impl std::error::Error for DartError<'_> {}

impl<'a> DartError<'a> {
    pub fn error_string(&self) -> &'a str {
        // Safe:
        // - we know it's a valid handle
        // - we make sure it doesn't leak out of it's validity context
        unsafe {
            let cstr = CStr::from_ptr(fpslot!(@call Dart_GetError_DL(self.0.handle)));
            std::str::from_utf8(cstr.to_bytes()).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_not_impl_any;

    use super::*;

    #[test]
    fn test_static_bounds() {
        assert_not_impl_any!(DartHandle: Send, Sync);
    }
}
