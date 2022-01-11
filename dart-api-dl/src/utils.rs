// Copyright 2022 Xayn AG
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

use std::{convert::TryInto, process::abort, ptr::NonNull};

/// Turns a pointer and length valid for a rust slice for a dart array pointer and length.
///
/// If a nullptr is passed in `NonNull::dangle()` is returned as pointer for the
/// zero length slice.
///
/// # Safety
///
/// The `ptr` must either be a null pointer or one which is
/// valid for creating a slice of `length` element of type `T`.
///
/// See [`std::slice::from_raw_parts_mut`].
///
/// # Abort
///
/// - Aborts if `len < 0` and it's not a null pointer.
/// - Aborts if `len > 0` and it's a null pointer.
///
/// In both cases there is some serious bug in the dart vm, while on
/// itself panicking would be better as we are in FFI code and not
/// necessary inside of a `catch_unwind` block we do not want to
/// do so.
pub(crate) unsafe fn prepare_dart_array_parts<T>(ptr: *const T, len: isize) -> (*const T, usize) {
    let len = len.try_into().unwrap_or_else(|_| abort());
    if (len == 0) != ptr.is_null() {
        abort()
    }
    let ptr = if ptr.is_null() {
        NonNull::dangling().as_ptr()
    } else {
        ptr
    };
    (ptr, len)
}

/// See [`prepare_dart_array_parts()`].
pub(crate) unsafe fn prepare_dart_array_parts_mut<T>(ptr: *mut T, len: isize) -> (*mut T, usize) {
    let len = len.try_into().unwrap_or_else(|_| abort());
    if (len == 0) != ptr.is_null() {
        abort()
    }
    let ptr = if ptr.is_null() {
        NonNull::dangling().as_ptr()
    } else {
        ptr
    };
    (ptr, len)
}
