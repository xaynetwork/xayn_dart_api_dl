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
    convert::{TryFrom, TryInto},
    ffi::{c_void, CString, NulError},
};

use dart_api_dl_sys::{
    Dart_CObject,
    Dart_CObject_Type,
    _Dart_CObject__bindgen_ty_1,
    _Dart_CObject__bindgen_ty_1__bindgen_ty_1,
    _Dart_CObject__bindgen_ty_1__bindgen_ty_2,
    _Dart_CObject__bindgen_ty_1__bindgen_ty_3,
};

use crate::{ports::SendPort, utils::prepare_dart_array_parts_mut};

use super::{CObjectMut, Capability, CustomExternalTyped, TypedData};

/// Wrapper around a [`Dart_CObject`] which is owned by rust.
//FIXME impl debug when we add a `CObjectRef` with a `value_ref()` method.
#[repr(transparent)]
pub struct CObject(Dart_CObject);

impl CObject {
    /// Create a [`CObjectMut`].
    ///
    /// Be aware that this acts mostly like a read-only reference but due to
    /// the way dart works it requires a `&mut` borrow.
    pub fn as_mut(&mut self) -> CObjectMut<'_> {
        CObjectMut {
            partial_mut: &mut self.0,
        }
    }

    /// Create a [`CObject`] containing null.
    pub fn null() -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kNull,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: false },
        })
    }

    /// Create a [`CObject`] containing a bool.
    pub fn bool(val: bool) -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kBool,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: val },
        })
    }

    /// Create a [`CObject`] containing a 32bit signed int.
    pub fn int32(val: i32) -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt32,
            value: _Dart_CObject__bindgen_ty_1 { as_int32: val },
        })
    }

    /// Create a [`CObject`] containing a 64bit signed int.
    pub fn int64(val: i64) -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt64,
            value: _Dart_CObject__bindgen_ty_1 { as_int64: val },
        })
    }

    /// Create a [`CObject`] containing a 64bit float.
    pub fn double(val: f64) -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kDouble,
            value: _Dart_CObject__bindgen_ty_1 { as_double: val },
        })
    }

    /// Create a [`CObject`] containing a string.
    ///
    /// This clones the string.
    ///
    /// # Errors
    ///
    /// If the string contains `0` bytes an error is returned.
    pub fn string(val: impl AsRef<str>) -> Result<Self, NulError> {
        let val = CString::new(val.as_ref())?;
        Ok(Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 {
                as_string: val.into_raw(),
            },
        }))
    }

    /// Create a [`CObject`] containing a string.
    ///
    /// Like [`CObject::string()`], but cuts off when encountering a `'\0'`.
    pub fn string_lossy(val: impl AsRef<str>) -> Self {
        let bytes = val.as_ref().as_bytes();
        let end_idx = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        //Safe we just did the checks
        let c_string = unsafe { CString::from_vec_unchecked(bytes[..end_idx].to_owned()) };
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 {
                as_string: c_string.into_raw(),
            },
        })
    }

    /// Create a [`CObject`] containing a [`SendPort`].
    pub fn send_port(port: SendPort) -> Self {
        let (id, origin_id) = port.as_raw();
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kSendPort,
            value: _Dart_CObject__bindgen_ty_1 {
                as_send_port: _Dart_CObject__bindgen_ty_1__bindgen_ty_1 { id, origin_id },
            },
        })
    }

    /// Create a [`CObject`] containing a [`Capability`].
    pub fn capability(id: Capability) -> Self {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kCapability,
            value: _Dart_CObject__bindgen_ty_1 {
                as_capability: _Dart_CObject__bindgen_ty_1__bindgen_ty_2 { id },
            },
        })
    }

    /// Create a [`CObject`] containing an array of boxed [`CObject`]'s.
    #[allow(clippy::vec_box)]
    pub fn array(array: Vec<Box<CObject>>) -> Self {
        let bs = array.into_boxed_slice();
        // We can't really have an array.len() > isize::MAX here, but we
        // really don't want to panic.
        let len = bs.len().try_into().unwrap_or(isize::MAX);
        // SAFE: as CObject is repr(transparent) as such `Box<CObject>` and `*mut Dart_CObject` have same layout.
        let ptr = Box::into_raw(bs).cast::<*mut Dart_CObject>();
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kArray,
            value: _Dart_CObject__bindgen_ty_1 {
                as_array: _Dart_CObject__bindgen_ty_1__bindgen_ty_3 {
                    length: len,
                    values: ptr,
                },
            },
        })
    }

    /// Create a [`CObject`] containing typed data.
    ///
    /// This will for now internally delegate to creating external
    /// typed data. This is an implementational detail **which might
    /// change**.
    ///
    /// Use [`CObject::external_typed_data()`] instead if you want
    /// to rely on it's performance characteristics.
    pub fn typed_data(data: TypedData) -> Self {
        Self::external_typed_data(data)
    }

    /// Create a [`CObject`] containing a .
    pub fn external_typed_data<CET>(data: CET) -> Self
    where
        CET: CustomExternalTyped,
    {
        Self(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kExternalTypedData,
            value: _Dart_CObject__bindgen_ty_1 {
                //Safe: due to the unsafe contract on CustomExternalTyped
                as_external_typed_data: data.into_external_typed_data(),
            },
        })
    }
}

impl Drop for CObject {
    fn drop(&mut self) {
        match self.0.type_ {
            Dart_CObject_Type::Dart_CObject_kNull
            | Dart_CObject_Type::Dart_CObject_kBool
            | Dart_CObject_Type::Dart_CObject_kInt32
            | Dart_CObject_Type::Dart_CObject_kInt64
            | Dart_CObject_Type::Dart_CObject_kDouble
            | Dart_CObject_Type::Dart_CObject_kCapability
            | Dart_CObject_Type::Dart_CObject_kSendPort => { /*nothing to do*/ }
            Dart_CObject_Type::Dart_CObject_kString => {
                drop(unsafe { CString::from_raw(self.0.value.as_string) });
            }
            Dart_CObject_Type::Dart_CObject_kArray => drop(unsafe {
                let (ptr, len) = prepare_dart_array_parts_mut(
                    self.0.value.as_array.values,
                    self.0.value.as_array.length,
                );
                Vec::from_raw_parts(ptr, len, len)
            }),
            Dart_CObject_Type::Dart_CObject_kExternalTypedData => {
                // we can only hit this if we didn't send it, in
                // which case we can drop it.
                // Safe:
                // - we just call the finalization handler
                unsafe {
                    let etd = &self.0.value.as_external_typed_data;
                    let data = etd.data;
                    let peer = etd.peer;
                    let callback = etd.callback;
                    (callback.expect("unexpected null pointer callback"))(
                        data.cast::<c_void>(),
                        peer,
                    );
                }
            }
            _ => {
                // also panics on: Dart_CObject_Type::Dart_CObject_kTypedData
                // we currently don't create it so we can't reach a drop with it
                unimplemented!("unsupported `CObject` format");
            }
        }
    }
}

impl Default for CObject {
    fn default() -> Self {
        Self::null()
    }
}

macro_rules! impl_from {
    ($($t:ty => $c:ident);* $(;)?) => ($(
        impl From<$t> for CObject {
            fn from(v: $t) -> Self {
                CObject::$c(v)
            }
        }
    )*);
}

impl_from!(
    bool => bool;
    i32 => int32;
    i64 => int64;
    SendPort => send_port;
    Vec<Box<CObject>> => array;
    TypedData => typed_data;
);

impl TryFrom<String> for CObject {
    type Error = NulError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        CObject::string(value)
    }
}
