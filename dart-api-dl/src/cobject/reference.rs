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

use std::{
    convert::TryInto,
    ffi::CStr,
    fmt::{self, Debug},
    slice,
};

use dart_api_dl_sys::{Dart_CObject, Dart_CObject_Type};

use crate::{ports::SendPort, DartRuntime};

use super::{
    CObjectRef,
    CObjectType,
    Capability,
    TypedDataRef,
    TypedDataType,
    UnknownCObjectType,
    UnknownTypedDataType,
};

/// Wrapper around a `Dart_CObject` which can be read, but which we do not own.
///
/// As such we can't deallocate anything in it and should in general not modify it.
// Transparent repr is very important as we will "unsafe" cast between the dart type
// and our new-type which we use to attach methods and safety to the dart type.
#[repr(transparent)]
pub struct CObject(pub(super) Dart_CObject);

impl CObject {
    /// Cast a pointer to a [`Dart_CObject`] to a [`CObject`] for the duration of the closure.
    ///
    /// # Safety
    ///
    /// 1. the pointer must point to a valid [`Dart_CObject`]
    /// 2. the [`Dart_CObject`] must be sound/consistent, e.g.
    ///    the type and set data match and are sound. This
    ///    moves the unsafety of the various `as_*` methods
    ///    into this method.
    pub unsafe fn with_pointer<R>(
        ptr: *mut Dart_CObject,
        func: impl for<'a> FnOnce(&'a mut CObject) -> R,
    ) -> R {
        func(unsafe { &mut *ptr.cast::<CObject>() })
    }

    /// Return the underlying pointer.
    ///
    /// # Safety
    ///
    /// If you use unsafe code to modify the underlying object
    /// you MUST make sure it still is sound and that you do
    /// not provoke use-after free or double free situations.
    ///
    /// Preferably do not modify the object at all.
    pub fn as_mut_ptr(&mut self) -> *mut Dart_CObject {
        &mut self.0
    }

    /// Set type to null, doesn't run any drop and as such might leak memory.
    pub(crate) fn set_to_null(&mut self) {
        self.0.type_ = Dart_CObject_Type::Dart_CObject_kNull;
    }

    /// Returns the type (tag/variant) of the [`CObject`].
    ///
    /// # Errors
    ///
    /// Fails if the type is not known (supported) by this library.
    pub fn r#type(&self) -> Result<CObjectType, UnknownCObjectType> {
        self.0.type_.try_into()
    }

    /// Returns `Some` if the object is null.
    pub fn as_null(&self, rt: DartRuntime) -> Option<()> {
        if let Ok(CObjectRef::Null) = self.value_ref(rt) {
            Some(())
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a bool.
    pub fn as_bool(&self, rt: DartRuntime) -> Option<bool> {
        if let Ok(CObjectRef::Bool(b)) = self.value_ref(rt) {
            Some(b)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a 32bit int.
    pub fn as_int32(&self, rt: DartRuntime) -> Option<i32> {
        if let Ok(CObjectRef::Int32(v)) = self.value_ref(rt) {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a 64bit int.
    pub fn as_int64(&self, rt: DartRuntime) -> Option<i64> {
        if let Ok(CObjectRef::Int64(v)) = self.value_ref(rt) {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a 32bit or 64bit int.
    pub fn as_int(&self, rt: DartRuntime) -> Option<i64> {
        self.as_int32(rt)
            .map_or_else(|| self.as_int64(rt), |v| Some(v.into()))
    }

    /// Returns `Some` if the object is a 64bit float.
    pub fn as_double(&self, rt: DartRuntime) -> Option<f64> {
        if let Ok(CObjectRef::Double(d)) = self.value_ref(rt) {
            Some(d)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a string.
    pub fn as_string(&self, rt: DartRuntime) -> Option<&str> {
        if let Ok(CObjectRef::String(s)) = self.value_ref(rt) {
            Some(s)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is an array of references to [`CObject`]s.
    pub fn as_array(&self, rt: DartRuntime) -> Option<&[&CObject]> {
        if let Ok(CObjectRef::Array(array)) = self.value_ref(rt) {
            Some(array)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is typed data.
    ///
    /// It's `Some((Ok(), _))` if it's typed data of a typed data
    /// variant which is supported by this library.
    ///
    /// It's `Some(_, true)` if it's externally typed data, normally
    /// if it's externally or not-externally typed data doesn't make
    /// a difference for the consumer.
    pub fn as_typed_data(
        &self,
        rt: DartRuntime,
    ) -> Option<(Result<TypedDataRef<'_>, UnknownTypedDataType>, bool)> {
        if let Ok(CObjectRef::TypedData {
            data,
            external_typed,
        }) = self.value_ref(rt)
        {
            Some((data, external_typed))
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a send port.
    ///
    /// As we can send an `ILLEGAL_PORT` we can have an object which
    /// is a send port variant but doesn't contain a `SendPort` as
    /// such it's an `Option<Option<>>`.
    #[allow(clippy::option_option)]
    pub fn as_send_port(&self, rt: DartRuntime) -> Option<Option<SendPort>> {
        if let Ok(CObjectRef::SendPort(port)) = self.value_ref(rt) {
            Some(port)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a capability.
    pub fn as_capability(&self, rt: DartRuntime) -> Option<Capability> {
        if let Ok(CObjectRef::Capability(cap)) = self.value_ref(rt) {
            Some(cap)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is typed data.
    ///
    /// This is similar to [`CObject.as_typed_data()`] but only returns the typed
    /// data type.
    ///
    /// Returns `Some(Err(_))` if the typed data type isn't supported by this library.
    pub fn typed_data_type(&self) -> Option<Result<TypedDataType, UnknownTypedDataType>> {
        (self.0.type_ == Dart_CObject_Type::Dart_CObject_kTypedData
            || self.0.type_ == Dart_CObject_Type::Dart_CObject_kExternalTypedData)
            .then(|| {
                // Safe: We checked the the object type.
                unsafe { self.read_typed_data_type() }
            })
    }

    /// Reads the typed data type union field.
    ///
    /// # Safety
    ///
    /// Safe if the object type is either of:
    ///
    /// - `Dart_CObject_Type::Dart_CObject_kTypedData`
    /// - `Dart_CObject_Type::Dart_CObject_kExternalTypedData`
    unsafe fn read_typed_data_type(&self) -> Result<TypedDataType, UnknownTypedDataType> {
        // It's safe to always read from `as_typed_data` as `Dart_CObject` is intentionally
        // designed so that external typed data has the same fields in the same layout as
        // typed data (just some additional ones)
        unsafe { self.0.value.as_typed_data.type_ }.try_into()
    }

    /// If the type is known returns an enums with a type specific reference to the data.
    ///
    /// Copy types are provided as copy instead of a reference.
    ///
    /// All the `as_...` functions are based on this internally.
    ///
    /// # Errors
    ///
    /// If the object type is not supported an error is returned.
    pub fn value_ref(&self, rt: DartRuntime) -> Result<CObjectRef<'_>, UnknownCObjectType> {
        #![allow(clippy::enum_glob_use)]
        use CObjectRef::*;
        let r#type = self.r#type()?;
        match r#type {
            CObjectType::Null => Ok(Null),
            CObjectType::Bool => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Bool(unsafe { self.0.value.as_bool }))
            }
            CObjectType::Int32 => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Int32(unsafe { self.0.value.as_int32 }))
            }
            CObjectType::Int64 => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Int64(unsafe { self.0.value.as_int64 }))
            }
            CObjectType::Double => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Double(unsafe { self.0.value.as_double }))
            }
            CObjectType::String => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                // - strings in CObject are utf-8 (and 0 terminated)
                Ok(String(unsafe {
                    let c_str = CStr::from_ptr(self.0.value.as_string);
                    std::str::from_utf8_unchecked(c_str.to_bytes())
                }))
            }
            CObjectType::Array => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                // - ExternalTypedData is repr(transparent)
                // - *const/*mut/& all have the same representation
                Ok(Array(unsafe {
                    let ar = &self.0.value.as_array;
                    // *mut *mut Dart_CObject
                    let ptr = ar.values as *const &CObject;
                    // This runs in FFI so we really don't want to panic, so
                    // if length <0 we set length = 0 (which in itself is unsound).
                    slice::from_raw_parts(ptr, ar.length.try_into().unwrap_or(0))
                }))
            }
            CObjectType::TypedData | CObjectType::ExternalTypedData => {
                // Safe: We checked the object type.
                let data = unsafe { self.read_typed_data_type() }.map(|data_type| {
                    // Safe:
                    // - CObject is sound
                    // - we checked the type
                    unsafe {
                        let data = &self.0.value.as_typed_data;
                        TypedDataRef::from_raw(
                            data_type,
                            data.values as *const u8,
                            // This runs in FFI so we really don't want to panic, so
                            // if length <0 we set length = 0 (which in itself is unsound).
                            data.length.try_into().unwrap_or(0),
                        )
                    }
                });

                Ok(TypedData {
                    data,
                    external_typed: r#type == CObjectType::ExternalTypedData,
                })
            }
            CObjectType::SendPort => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(SendPort(unsafe {
                    let sp = &self.0.value.as_send_port;
                    rt.send_port_from_raw_with_origin(sp.id, sp.origin_id)
                }))
            }
            CObjectType::Capability => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Capability(unsafe { self.0.value.as_capability.id }))
            }
        }
    }
}

impl Debug for CObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(rt) = DartRuntime::instance() {
            f.debug_struct("ExternCObject")
                .field("as_enum", &self.value_ref(rt))
                .finish()
        } else {
            f.debug_struct("ExternCObject")
                .field("as_enum", &"<unknown>")
                .finish()
        }
    }
}
