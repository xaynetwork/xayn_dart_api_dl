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

use std::{convert::TryInto, ffi::c_void};

use dart_api_dl_sys::_Dart_CObject__bindgen_ty_1__bindgen_ty_5;

use crate::ports::SendPort;

use super::{CObjectRef, TypedDataType, UnknownTypedDataType};

/// External Typed Data as represented in a [`Dart_CObject`].
pub type ExternalTypedData = _Dart_CObject__bindgen_ty_1__bindgen_ty_5;

/// Dart Capability
pub type Capability = i64;

/// A reference to the data in the `CObject`.
///
/// In case of copy data a copy is used instead.
#[derive(Debug)]
pub enum CObjectValuesRef<'a> {
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
    Array(&'a [CObjectRef<'a>]),
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
    pub(super) unsafe fn from_raw(data_type: TypedDataType, data: *const u8, len: usize) -> Self {
        #![allow(
            unsafe_op_in_unsafe_fn,
            clippy::enum_glob_use,
            clippy::cast_ptr_alignment
        )]
        use std::slice::from_raw_parts;
        use TypedDataRef::*;
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

/// Owned typed data you can send to dart (through a [`CObject`]).
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

/// Hook to allow using custom external typed data.
///
/// # Safety
///
/// The returned external typed data must be sound to
/// use in a [`CObject`].
pub unsafe trait CustomExternalTyped {
    /// This should only be called by the `OwnedCObject` type.
    ///
    /// Directly dropping the return type of this function will
    /// leak the resources of this instance. Though `OwnedCObject`
    /// will make sure that this doesn't happen.
    fn into_external_typed_data(self) -> ExternalTypedData;
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
        $($st:ty = $typed_data_variant:ident),* $(,)?
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
