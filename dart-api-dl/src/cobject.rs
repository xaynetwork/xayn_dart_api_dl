//! This modules provides abstractions around `CObject`.
//!
//! The raw `Dart_CObject` does not only have some rust
//! unsafe types but also needs to be handles differently
//! depending on the context.
//!
//! As such we have multiple types:
//!
//! - [`CObject`] type which is read only.
//!   You will either get a reference to it
//!   from an external source or by dereferencing
//!   [`OwnedCObject`].
//!
//! - [`OwnedCObject`] a instance we created and as
//!   such we need to handle resource cleanup, like
//!   freeing allocated string.
//!
use std::{
    ffi::{c_void, CStr, CString, NulError},
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
    slice,
};

use dart_api_dl_sys::{
    Dart_CObject, Dart_CObject_Type, _Dart_CObject__bindgen_ty_1,
    _Dart_CObject__bindgen_ty_1__bindgen_ty_1, _Dart_CObject__bindgen_ty_1__bindgen_ty_2,
    _Dart_CObject__bindgen_ty_1__bindgen_ty_3, _Dart_CObject__bindgen_ty_1__bindgen_ty_5,
};

use crate::{port::SendPort, DartRuntime};

mod type_enums;

pub use type_enums::*;

/// Capability send as represented in a [`Dart_CObject`].
pub type Capability = i64;

/// External Typed Data as represented in a [`Dart_CObject`].
//TODO
pub type ExternalTypedData = _Dart_CObject__bindgen_ty_1__bindgen_ty_5;

/// Wrapper around a `Dart_CObject` which can be read, but which we do not own.
///
/// As such we can't deallocate anything in it and should in general not modify it.
///
// Transparent repr is very important as we will "unsafe" cast between the dart type
// and our new-type which we use to attach methods and safety to the dart type.
#[repr(transparent)]

pub struct CObject {
    obj: Dart_CObject,
}

impl CObject {
    /// Cast a pointer to a `Dart_CObject` to a `CObject` for the duration of the closure.
    ///
    /// # Safety
    ///
    /// 1. the pointer must point to a valid [`Dat_CObject`]
    /// 2. the [`Dat_CObject`] must be sound/consistent, e.g.
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
    /// not provoke can use-after free or double free situations.
    ///
    /// Preferable do not modify the object at all.
    pub fn as_ptr_mut(&mut self) -> *mut Dart_CObject {
        &mut self.obj
    }

    /// Set type to null, doesn't run any drop and as such might leak memory.
    pub(crate) fn set_to_null(&mut self) {
        self.obj.type_ = Dart_CObject_Type::Dart_CObject_kNull;
    }

    /// Returns the type (tag/variant) of the [`CObject`].
    ///
    /// # Errors
    ///
    /// Fails if the type is not known (supported) by this library.
    pub fn r#type(&self) -> Result<CObjectType, UnknownCObjectType> {
        self.obj.type_.try_into()
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

    /// Returns `Some` if the object is a array of references to [`CObject`]s.
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
    /// As we can send a `ILLEGAL_PORT` we can have a object which
    /// is a send port variant but doesn't contain a `SendPort` as
    /// such it's a `Option<Option<>>`.
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
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kTypedData
            || self.obj.type_ == Dart_CObject_Type::Dart_CObject_kExternalTypedData)
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
        unsafe { self.obj.value.as_typed_data.type_ }.try_into()
    }

    /// If the type is known returns a enums with a type specific reference to the data.
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
                Ok(Bool(unsafe { self.obj.value.as_bool }))
            }
            CObjectType::Int32 => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Int32(unsafe { self.obj.value.as_int32 }))
            }
            CObjectType::Int64 => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Int64(unsafe { self.obj.value.as_int64 }))
            }
            CObjectType::Double => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Double(unsafe { self.obj.value.as_double }))
            }
            CObjectType::String => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                // - strings in CObject are utf-8 (and 0 terminated)
                Ok(String(unsafe {
                    let c_str = CStr::from_ptr(self.obj.value.as_string);
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
                    let ar = &self.obj.value.as_array;
                    // *mut *mut Dart_CObject
                    let ptr = ar.values as *const &CObject;
                    // This runs in FFI so we really don't want to panic, so
                    // if length <0 we set length = 0 (which in itself is unsound).
                    slice::from_raw_parts(ptr, ar.length.try_into().unwrap_or(0))
                }))
            }
            CObjectType::TypedData | CObjectType::ExternalTypedData => {
                //Safe: We checked the object type.
                let type_res = unsafe { self.read_typed_data_type() };
                let data = type_res.map(|data_type| {
                    // Safe:
                    // - CObject is sound
                    // - we checked the type
                    unsafe {
                        let data = &self.obj.value.as_typed_data;
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
                    let sp = &self.obj.value.as_send_port;
                    rt.send_port_from_raw_with_origin(sp.id, sp.origin_id)
                }))
            }
            CObjectType::Capability => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(Capability(unsafe { self.obj.value.as_capability.id }))
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

/// A reference to the data in the `CObject`.
///
/// In case of copy data a copy is used instead.
#[derive(Debug)]
pub enum CObjectRef<'a> {
    /// The object is null.
    Null,
    /// The object is a bool.
    Bool(bool),
    /// The object is a 32bit int.
    Int32(i32),
    /// The object is a 64bit int.
    Int64(i64),
    /// The object is a 64bit float.
    Double(f64),
    /// The object is a string.
    String(&'a str),
    /// The object is an array of `CObject` references.
    Array(&'a [&'a CObject]),
    /// The object is a typed data.
    TypedData {
        /// `Ok` if the data is of a supported typed data type.
        data: Result<TypedDataRef<'a>, UnknownTypedDataType>,
        /// Hints if the data was externally typed or not.
        external_typed: bool,
    },
    /// The object is a send port variant. `Some` if the port is not the `ILLEGAL_PORT`.
    SendPort(Option<SendPort>),
    /// The object is a capability.
    Capability(Capability),
}

/// Reference to typed data in a `CObject`.
#[derive(Debug, Clone, Copy)]
pub enum TypedDataRef<'a> {
    /// `u8` data, for rust the same as `Uint8` and `Uint8Clamped`.
    ///
    /// In dart this is represented as a fixed sized buffer.
    ByteData(&'a [u8]),
    /// `i8` data
    Int8(&'a [i8]),
    /// `u8` data, for rust the same as `ByteData` and `Uint8Clamped`.
    Uint8(&'a [u8]),
    /// `u8` data, for rust the same as `ByteData` and `Uint8Clamped`.
    ///
    /// In dart a list of this type will clamp integers outside of
    /// the `u8` range when inserting them (instead of using the lower
    /// most byte).
    Uint8Clamped(&'a [u8]),
    /// `i16` data
    Int16(&'a [i16]),
    /// `u16` data
    Uint16(&'a [u16]),
    /// `i32` data
    Int32(&'a [i32]),
    /// `u32` data
    Uint32(&'a [u32]),
    /// `i64` data
    Int64(&'a [i64]),
    /// `u64` data
    Uint64(&'a [u64]),
    /// `f32` data
    Float32(&'a [f32]),
    /// `f64` data
    Float64(&'a [f64]),
    /// Data representing 4 `i32`s, which i32 maps to which field in dart isn't well defined.
    Int32x4(&'a [[i32; 4]]),
    /// Data representing 4 `i32`s, which i32 maps to which field in dart isn't well defined.
    Float32x4(&'a [[f32; 4]]),
    /// Data representing 4 `i32`s, which i32 maps to which field in dart isn't well defined.
    Float64x2(&'a [[f64; 2]]),
}

impl TypedDataRef<'_> {
    unsafe fn from_raw(data_type: TypedDataType, data: *const u8, len: usize) -> Self {
        #![allow(unsafe_op_in_unsafe_fn, clippy::enum_glob_use, clippy::cast_ptr_alignment)]
        use TypedDataRef::*;
        use std::slice::from_raw_parts;
        match data_type {
            TypedDataType::ByteData => ByteData(from_raw_parts(data, len)),
            TypedDataType::Int8 => Int8(from_raw_parts(data.cast::<i8>(), len)),
            TypedDataType::Uint8 => Uint8(from_raw_parts(data, len)),
            TypedDataType::Uint8Clamped => Uint8Clamped(from_raw_parts(data, len)),
            TypedDataType::Int16 => Int16(from_raw_parts(data.cast::<i16>(), len)),
            TypedDataType::Uint16 => Uint16(from_raw_parts(data.cast::<u16>(), len)),
            TypedDataType::Int32 => Int32(from_raw_parts(data.cast::<i32>(), len)),
            TypedDataType::Uint32 => Uint32(from_raw_parts(data.cast::<u32>(), len)),
            TypedDataType::Int64 => Int64(from_raw_parts(data.cast::<i64>(), len)),
            TypedDataType::Uint64 => Uint64(from_raw_parts(data.cast::<u64>(), len)),
            TypedDataType::Float32 => Float32(from_raw_parts(data.cast::<f32>(), len)),
            TypedDataType::Float64 => Float64(from_raw_parts(data.cast::<f64>(), len)),
            TypedDataType::Int32x4 => Int32x4(from_raw_parts(data.cast::<[i32; 4]>(), len)),
            TypedDataType::Float32x4 => Float32x4(from_raw_parts(data.cast::<[f32; 4]>(), len)),
            TypedDataType::Float64x2 => Float64x2(from_raw_parts(data.cast::<[f64; 2]>(), len)),
        }
    }
}

/// Owned typed data you can send to dart (through a `OwnedCObject`).
#[derive(Debug, Clone)]
pub enum TypedData {
    /// A boxed slice of bytes.
    ByteData(Box<[u8]>),
    /// A vector of `i8`s.
    Int8(Vec<i8>),
    /// A vector of `u8`s.
    Uint8(Vec<u8>),
    /// A vector of `u8`s which will be represented through a clamping container in dart.
    Uint8Clamped(Vec<u8>),
    /// A vector of `i16`s.
    Int16(Vec<i16>),
    /// A vector of `u16`s.
    Uint16(Vec<u16>),
    /// A vector of `i32`s.
    Int32(Vec<i32>),
    /// A vector of `u32`s.
    Uint32(Vec<u32>),
    /// A vector of `i64`s.
    Int64(Vec<i64>),
    /// A vector of `u64`s.
    Uint64(Vec<u64>),
    /// A vector of `f32`s.
    Float32(Vec<f32>),
    /// A vector of `f64`s.
    Float64(Vec<f64>),
    /// A vector of 4 `i32`s per element.
    Int32x4(Vec<[i32; 4]>),
    /// A vector of 4 `f32`s per element.
    Float32x4(Vec<[f32; 4]>),
    /// A vector of 2 `f64`s per element.
    Float64x2(Vec<[f64; 2]>),
}

impl TypedData {
    /// Returns the data type of this typed data.
    pub fn data_type(&self) -> TypedDataType {
        match self {
            TypedData::ByteData(_) => TypedDataType::ByteData,
            TypedData::Int8(_) => TypedDataType::Int8,
            TypedData::Uint8(_) => TypedDataType::Uint8,
            TypedData::Uint8Clamped(_) => TypedDataType::Uint8Clamped,
            TypedData::Int16(_) => TypedDataType::Int16,
            TypedData::Uint16(_) => TypedDataType::Uint16,
            TypedData::Int32(_) => TypedDataType::Int32,
            TypedData::Uint32(_) => TypedDataType::Uint32,
            TypedData::Int64(_) => TypedDataType::Int64,
            TypedData::Uint64(_) => TypedDataType::Uint64,
            TypedData::Float32(_) => TypedDataType::Float32,
            TypedData::Float64(_) => TypedDataType::Float64,
            TypedData::Int32x4(_) => TypedDataType::Int32x4,
            TypedData::Float32x4(_) => TypedDataType::Float32x4,
            TypedData::Float64x2(_) => TypedDataType::Float64x2,
        }
    }
}

unsafe impl CustomExternalTyped for TypedData {
    fn into_external_typed_data(self) -> ExternalTypedData {
        match self {
            TypedData::ByteData(mut data) => {
                let ptr = data.as_mut_ptr().cast::<u8>();
                let length = data.len().try_into().unwrap();
                let peer = Box::into_raw(Box::new(data)).cast::<c_void>();

                ExternalTypedData {
                    type_: TypedDataType::ByteData.into(),
                    length,
                    data: ptr,
                    peer,
                    callback: Some(drop_boxed_peer::<Box<[u8]>>),
                }
            }
            TypedData::Int8(data) => data.into_external_typed_data(),
            TypedData::Uint8(data) => data.into_external_typed_data(),
            TypedData::Uint8Clamped(mut data) => {
                let ptr = data.as_mut_ptr().cast::<u8>();
                let length = data.len().try_into().unwrap();
                let peer = Box::into_raw(Box::new(data)).cast::<c_void>();

                ExternalTypedData {
                    type_: TypedDataType::ByteData.into(),
                    length,
                    data: ptr,
                    peer,
                    callback: Some(drop_boxed_peer::<Box<[u8]>>),
                }
            }
            TypedData::Int16(data) => data.into_external_typed_data(),
            TypedData::Uint16(data) => data.into_external_typed_data(),
            TypedData::Int32(data) => data.into_external_typed_data(),
            TypedData::Uint32(data) => data.into_external_typed_data(),
            TypedData::Int64(data) => data.into_external_typed_data(),
            TypedData::Uint64(data) => data.into_external_typed_data(),
            TypedData::Float32(data) => data.into_external_typed_data(),
            TypedData::Float64(data) => data.into_external_typed_data(),
            TypedData::Int32x4(data) => data.into_external_typed_data(),
            TypedData::Float32x4(data) => data.into_external_typed_data(),
            TypedData::Float64x2(data) => data.into_external_typed_data(),
        }
    }
}

macro_rules! impl_custom_external_typed_data_for_vec {
    (unsafe impl for {
        $($st:ty = $typed_data_variant:ident,)*
    }) => ($(
        unsafe impl CustomExternalTyped for Vec<$st> {
            fn into_external_typed_data(mut self) -> ExternalTypedData {
                let data = self.as_mut_ptr() as *mut u8;
                let length = self.len().try_into().unwrap();
                let peer = Box::into_raw(Box::new(self)) as *mut c_void;

                ExternalTypedData {
                    type_:  TypedDataType::$typed_data_variant.into(),
                    length,
                    data,
                    peer,
                    callback: Some(drop_boxed_peer::<Vec<$st>>),
                }
            }
        }
    )*);
}

impl_custom_external_typed_data_for_vec!(
    unsafe impl for {
        i8 = Int8,
        u8 = Uint8,
        i16 = Int16,
        u16 = Uint16,
        i32 = Int32,
        u32 = Uint32,
        i64 = Int64,
        u64 = Uint64,
        f32 = Float32,
        f64 = Float64,
        [i32; 4] = Int32x4,
        [f32; 4] = Float32x4,
        [f64; 2] = Float64x2,
    }
);

unsafe extern "C" fn drop_boxed_peer<T>(_data: *mut c_void, peer: *mut c_void) {
    drop(unsafe { Box::from_raw(peer.cast::<T>()) });
}

/// Wrapper around a [`CObject`] which is owned by rust.
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedCObject(CObject);

impl OwnedCObject {
    //not meant to be public, just a helper to reduce code duplication
    fn wrap_raw(obj: Dart_CObject) -> Self {
        Self(CObject { obj })
    }

    /// Create a [`CObject`] containing null.
    pub fn null() -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kNull,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: false },
        })
    }

    /// Create a [`CObject`] containing a bool.
    pub fn bool(val: bool) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kBool,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: val },
        })
    }

    /// Create a [`CObject`] containing a 32bit signed int.
    pub fn int32(val: i32) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt32,
            value: _Dart_CObject__bindgen_ty_1 { as_int32: val },
        })
    }

    /// Create a [`CObject`] containing a 64bit signed int.
    pub fn int64(val: i64) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt64,
            value: _Dart_CObject__bindgen_ty_1 { as_int64: val },
        })
    }

    /// Create a [`CObject`] containing a 64bit float.
    pub fn double(val: f64) -> Self {
        Self::wrap_raw(Dart_CObject {
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
    /// If the string contains a `0` bytes an error is returned.
    pub fn string(val: impl AsRef<str>) -> Result<Self, NulError> {
        let val = CString::new(val.as_ref())?;
        Ok(Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 {
                as_string: val.into_raw(),
            },
        }))
    }

    /// Create a [`CObject`] containing a string.
    ///
    /// Like [`CObject::string()`], but cut's of when encountering a `'\0'`.
    pub fn string_lossy(val: impl AsRef<str>) -> Self {
        let bytes = val.as_ref().as_bytes();
        let end_idx = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        //Safe we just did the checks
        let c_string = unsafe { CString::from_vec_unchecked(bytes[..end_idx].to_owned()) };
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 {
                as_string: c_string.into_raw(),
            },
        })
    }

    /// Create a [`CObject`] containing a [`SendPort`].
    pub fn send_port(port: SendPort) -> Self {
        let (id, origin_id) = port.as_raw();
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kSendPort,
            value: _Dart_CObject__bindgen_ty_1 {
                as_send_port: _Dart_CObject__bindgen_ty_1__bindgen_ty_1 { id, origin_id },
            },
        })
    }

    /// Create a [`CObject`] containing a [`Capability`].
    pub fn capability(id: i64) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kCapability,
            value: _Dart_CObject__bindgen_ty_1 {
                as_capability: _Dart_CObject__bindgen_ty_1__bindgen_ty_2 { id },
            },
        })
    }

    /// Create a [`CObject`] containing a array of boxed [`OwnedCObject`]'s.
    ///
    #[allow(clippy::vec_box)]
    pub fn array(array: Vec<Box<OwnedCObject>>) -> Self {
        let bs = array.into_boxed_slice();
        // We can't really have a array.len() > isize::MAX here, but we
        // don't really don't want to panic.
        let len = bs.len().try_into().unwrap_or(isize::MAX);
        // SAFE: as CObject is repr(transparent) as such `Box<CObject>` and `*mut Dart_CObject` have same layout.
        let ptr = Box::into_raw(bs).cast::<*mut Dart_CObject>();
        Self::wrap_raw(Dart_CObject {
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
    /// This will for now internally delegates to creating external
    /// typed data. This is an implementation details **which might
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
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kExternalTypedData,
            value: _Dart_CObject__bindgen_ty_1 {
                //Safe: due to the unsafe contract on CustomExternalTyped
                as_external_typed_data: data.into_external_typed_data(),
            },
        })
    }
}

impl Deref for OwnedCObject {
    type Target = CObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OwnedCObject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for OwnedCObject {
    fn drop(&mut self) {
        match self.obj.type_ {
            Dart_CObject_Type::Dart_CObject_kNull
            | Dart_CObject_Type::Dart_CObject_kBool
            | Dart_CObject_Type::Dart_CObject_kInt32
            | Dart_CObject_Type::Dart_CObject_kInt64
            | Dart_CObject_Type::Dart_CObject_kDouble
            | Dart_CObject_Type::Dart_CObject_kCapability
            | Dart_CObject_Type::Dart_CObject_kSendPort => { /*nothing to do*/ }
            Dart_CObject_Type::Dart_CObject_kString => {
                drop(unsafe { CString::from_raw(self.obj.value.as_string) });
                self.obj.type_ = Dart_CObject_Type::Dart_CObject_kNull;
            }
            Dart_CObject_Type::Dart_CObject_kArray => drop(unsafe {
                let len = self.obj.value.as_array.length.try_into().unwrap_or(0);
                let ptr = self.obj.value.as_array.values;
                Vec::from_raw_parts(ptr, len, len)
            }),
            Dart_CObject_Type::Dart_CObject_kExternalTypedData => {
                // we can only hit this if we didn't send it, in
                // which case we can drop it.
                // Safe:
                // - we just call the finalization handler
                unsafe {
                    let etd = &self.obj.value.as_external_typed_data;
                    let data = etd.data;
                    let peer = etd.peer;
                    let callback = etd.callback;
                    self.obj.type_ = Dart_CObject_Type::Dart_CObject_kNull;
                    (callback.expect("unexpected null pointer callback"))(
                        data.cast::<c_void>(),
                        peer,
                    );
                }
            }
            _ => {
                // also panics on: Dart_CObject_Type::Dart_CObject_kTypedData
                // we currently don't create it so we can't reach a drop with it
                panic!("unsupported `OwnedCObject` format");
            }
        }
    }
}

impl Default for OwnedCObject {
    fn default() -> Self {
        Self::null()
    }
}

/// Hook to allow using custom external typed data.
///
/// # Safety
///
/// The returned external typed data must be sound to
/// use in a [`CObject`].
///
pub unsafe trait CustomExternalTyped {
    /// This should only be called by the `OwnedCObject` type.
    ///
    /// Directly dropping the return type of this function will
    /// leak the resources of this instance. Through `OwnedCObject`
    /// will make sure that this doesn't happen.
    fn into_external_typed_data(self) -> ExternalTypedData;
}
