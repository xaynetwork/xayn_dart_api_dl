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

use crate::{
    ports::SendPort,
    utils::{prepare_dart_array_parts, prepare_dart_array_parts_mut},
    DartRuntime,
};

use super::{
    CObjectType,
    CObjectValuesRef,
    Capability,
    TypedDataRef,
    TypedDataType,
    UnknownCObjectType,
    UnknownTypedDataType,
};

/// Reference to a `Dart_CObject` that can be read but isn't own by rust.
///
/// Due to the design of dart's send port API this is a mutable reference,
/// and might be changed in some cases, e.g. when externally typed data is moved
/// to dart when sending it over a `SendPort`.
///
/// # A note about why that is not a `&mut` ref
///
/// `CObjectMut` might be owned by dart, so we should not arbitrarily
/// modify it. While dart uses a form of pooled GC and as such a modification
/// shouldn't normally have too bad consequences, with rust we don't have such mechanism,
/// which in turn could cause problems. Basically, we can at most move things out
/// of it but if we don't do that carefully, it might cause memory leaks.
///
/// More important we must not swap allocated values between `CObjects`. As swapping
/// between dart and rust owned objects, or between two object with different lifetimes
/// will cause soundness issues once at lest one of them is dropped.
///
/// As such we can't deallocate anything in it and should in general not modify it.
// Note: Transparent repr is very important as we will "unsafe" cast between the dart type
// and our new-type which we use to attach methods and safety to the dart type.
#[repr(transparent)]
pub struct CObjectMut<'a> {
    /// The reference to the raw `Dart_CObject`.
    ///
    /// # Safety
    ///
    /// It is only allowed to modify the referenced [`Dart_CObject`] by
    /// setting the external typed data to null or by temporary modifications
    /// made by dart when sending it via the port.
    ///
    /// Those guarantees are similar to `Pin` but less strict.
    ///
    /// You could say that this is basically a `&Dart_CObject` except that
    /// externally typed data is set to null when it has been moved out and
    /// the fact that sending requires a mut ref for tmp. in place
    /// modifications that dart does as a form of optimization.
    pub(super) partial_mut: &'a mut Dart_CObject,
}

impl<'a> CObjectMut<'a> {
    /// Cast a pointer from a [`Dart_CObject`] to a [`CObjectMut`] for the duration of the closure.
    ///
    /// # Safety
    ///
    /// 1. the pointer must point to a valid [`Dart_CObject`]
    /// 2. the [`Dart_CObject`] must be sound/consistent, e.g.
    ///    the type and set data match and are sound. This
    ///    moves the unsafety of the various `as_*` methods
    ///    into this method.
    /// 3. it must be valid to turn the pointer into a `&mut`
    ///    for the duration of this function call
    pub unsafe fn with_pointer<R>(
        ptr: *mut Dart_CObject,
        func: impl for<'b> FnOnce(CObjectMut<'b>) -> R,
    ) -> R {
        func(unsafe {
            CObjectMut {
                partial_mut: &mut *ptr,
            }
        })
    }

    /// Reborrows this instance.
    pub fn reborrow(&mut self) -> CObjectMut<'_> {
        CObjectMut {
            partial_mut: self.partial_mut,
        }
    }

    /// Return the underlying pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer must only be used for sending it
    /// to a port. If the [`Dart_CObject`] contains external typed data,
    /// this data must be removed after sending by setting it to null.
    ///
    /// The `SendPort` abstraction provided by this library does so automatically.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut Dart_CObject {
        self.partial_mut
    }

    /// Set type to null, doesn't run any drop and as such might leak memory.
    pub(crate) fn set_to_null(&mut self) {
        self.partial_mut.type_ = Dart_CObject_Type::Dart_CObject_kNull;
    }

    /// Returns the type (tag/variant) of the [`CObjectMut`].
    ///
    /// # Errors
    ///
    /// Fails if the type is not known (supported) by this library.
    pub fn r#type(&self) -> Result<CObjectType, UnknownCObjectType> {
        self.partial_mut.type_.try_into()
    }

    /// Returns `Some` if the object is null.
    pub fn as_null(&self, rt: DartRuntime) -> Option<()> {
        if let Ok(CObjectValuesRef::Null) = self.value_ref(rt) {
            Some(())
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a bool.
    pub fn as_bool(&self, rt: DartRuntime) -> Option<bool> {
        if let Ok(CObjectValuesRef::Bool(b)) = self.value_ref(rt) {
            Some(b)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a 32bit int.
    pub fn as_int32(&self, rt: DartRuntime) -> Option<i32> {
        if let Ok(CObjectValuesRef::Int32(v)) = self.value_ref(rt) {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a 64bit int.
    pub fn as_int64(&self, rt: DartRuntime) -> Option<i64> {
        if let Ok(CObjectValuesRef::Int64(v)) = self.value_ref(rt) {
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
        if let Ok(CObjectValuesRef::Double(d)) = self.value_ref(rt) {
            Some(d)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a string.
    pub fn as_string(&self, rt: DartRuntime) -> Option<&str> {
        if let Ok(CObjectValuesRef::String(s)) = self.value_ref(rt) {
            Some(s)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is an array of references to [`CObjectMut`]s.
    pub fn as_array(&self, rt: DartRuntime) -> Option<&[CObjectMut<'_>]> {
        if let Ok(CObjectValuesRef::Array(array)) = self.value_ref(rt) {
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
        if let Ok(CObjectValuesRef::TypedData {
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
        if let Ok(CObjectValuesRef::SendPort(port)) = self.value_ref(rt) {
            Some(port)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is a capability.
    pub fn as_capability(&self, rt: DartRuntime) -> Option<Capability> {
        if let Ok(CObjectValuesRef::Capability(cap)) = self.value_ref(rt) {
            Some(cap)
        } else {
            None
        }
    }

    /// Returns `Some` if the object is typed data.
    ///
    /// This is similar to [`CObjectMut.as_typed_data()`] but only returns the typed
    /// data type.
    ///
    /// Returns `Some(Err(_))` if the typed data type isn't supported by this library.
    pub fn typed_data_type(&self) -> Option<Result<TypedDataType, UnknownTypedDataType>> {
        (self.partial_mut.type_ == Dart_CObject_Type::Dart_CObject_kTypedData
            || self.partial_mut.type_ == Dart_CObject_Type::Dart_CObject_kExternalTypedData)
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
        unsafe { self.partial_mut.value.as_typed_data.type_ }.try_into()
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
    pub fn value_ref(&self, rt: DartRuntime) -> Result<CObjectValuesRef<'_>, UnknownCObjectType> {
        #![allow(clippy::enum_glob_use)]
        use CObjectValuesRef::*;
        let r#type = self.r#type()?;
        match r#type {
            CObjectType::Null => Ok(Null),
            CObjectType::Bool => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(Bool(unsafe { self.partial_mut.value.as_bool }))
            }
            CObjectType::Int32 => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(Int32(unsafe { self.partial_mut.value.as_int32 }))
            }
            CObjectType::Int64 => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(Int64(unsafe { self.partial_mut.value.as_int64 }))
            }
            CObjectType::Double => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(Double(unsafe { self.partial_mut.value.as_double }))
            }
            CObjectType::String => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                // - strings in CObject are utf-8 (and 0 terminated)
                Ok(String(unsafe {
                    let c_str = CStr::from_ptr(self.partial_mut.value.as_string);
                    std::str::from_utf8_unchecked(c_str.to_bytes())
                }))
            }
            CObjectType::Array => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                // - ExternalTypedData is repr(transparent)
                // - *const/*mut/& all have the same representation
                Ok(Array(unsafe {
                    let as_array = &self.partial_mut.value.as_array;
                    let (ptr, len) = prepare_dart_array_parts(
                        // *mut *mut Dart_CObject
                        as_array.values.cast::<CObjectMut<'a>>(),
                        as_array.length,
                    );
                    slice::from_raw_parts(ptr, len)
                }))
            }
            CObjectType::TypedData | CObjectType::ExternalTypedData => {
                // Safe: We checked the object type.
                let data = unsafe { self.read_typed_data_type() }.map(|data_type| {
                    // Safe:
                    // - the CObject behind the reference is sound
                    // - we checked the type
                    unsafe {
                        let as_typed_data = &self.partial_mut.value.as_typed_data;
                        let (ptr, len) =
                            prepare_dart_array_parts(as_typed_data.values, as_typed_data.length);
                        TypedDataRef::from_raw(data_type, ptr, len)
                    }
                });

                Ok(TypedData {
                    data,
                    external_typed: r#type == CObjectType::ExternalTypedData,
                })
            }
            CObjectType::SendPort => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(SendPort(unsafe {
                    let sp = &self.partial_mut.value.as_send_port;
                    rt.send_port_from_raw_with_origin(sp.id, sp.origin_id)
                }))
            }
            CObjectType::Capability => {
                // Safe:
                // - the CObject behind the reference is sound
                // - we checked the type
                Ok(Capability(unsafe {
                    self.partial_mut.value.as_capability.id
                }))
            }
        }
    }

    pub(crate) fn null_external_typed_objects(&mut self, rt: DartRuntime) {
        match self.r#type() {
            Ok(CObjectType::ExternalTypedData) => self.set_to_null(),
            Ok(CObjectType::Array) => {
                let array = unsafe {
                    let as_array = &mut self.partial_mut.value.as_array;
                    let (ptr, len) = prepare_dart_array_parts_mut(
                        // *mut *mut Dart_CObject
                        as_array.values.cast::<CObjectMut<'a>>(),
                        as_array.length,
                    );
                    slice::from_raw_parts_mut(ptr, len)
                };
                for element in array {
                    element.null_external_typed_objects(rt);
                }
            }
            _ => {}
        }
    }
}

impl Debug for CObjectMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(rt) = DartRuntime::instance() {
            f.debug_struct("CObjectMut")
                .field("as_enum", &self.value_ref(rt))
                .finish()
        } else {
            f.debug_struct("CObjectMut")
                .field("as_enum", &"<unknown>")
                .finish()
        }
    }
}
