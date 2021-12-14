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

use dart_api_dl_sys::{Dart_CObject_Type, Dart_TypedData_Type};
use thiserror::Error;

macro_rules! impl_from_to_pseudo_enums {
    ($(#[$attr:meta])* pub enum $enum_name:ident from $native_name:ident {
        type Error = $error:ident;
        $($enum_variant:ident = $native_const:ident,)*
    }

    ) => (
        $(#[$attr])*
        pub enum $enum_name {
            $(#[allow(missing_docs)] $enum_variant,)*
        }

        impl std::convert::TryFrom<$native_name> for $enum_name {
            type Error = $error;
            fn try_from(v: $native_name) -> Result<Self, Self::Error> {
                Ok(match v {
                    $($native_name::$native_const => $enum_name::$enum_variant,)*
                    v => return Err($error(v)),
                })
            }
        }

        impl From<$enum_name> for $native_name {
            fn from(v: $enum_name) -> Self {
                match v {
                    $($enum_name::$enum_variant => $native_name::$native_const,)*
                }
            }
        }
    );
}

impl_from_to_pseudo_enums! {
    /// Supported types of [`CObject`](crate::cobject::CObject)s.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum CObjectType from Dart_CObject_Type {
        type Error = UnknownCObjectType;
        Null = Dart_CObject_kNull,
        Bool = Dart_CObject_kBool,
        Int32 = Dart_CObject_kInt32,
        Int64 = Dart_CObject_kInt64,
        Double = Dart_CObject_kDouble,
        String = Dart_CObject_kString,
        Array = Dart_CObject_kArray,
        TypedData = Dart_CObject_kTypedData,
        ExternalTypedData = Dart_CObject_kExternalTypedData,
        SendPort = Dart_CObject_kSendPort,
        Capability = Dart_CObject_kCapability,
    }
}

/// The [`CObjectType`] isn't known/supported by this library.
///
/// There are a few cases where a type is not supported:
///
/// - It was added in a newer Dart VM version.
/// - It's the `Dart_CObject_kUnsupported` type.
/// - It's the `Dart_CObject_kNumberOfTypes` type.
#[derive(Debug, Error, PartialEq)]
#[error("UnknownCObjectType: {:?}", _0)]
pub struct UnknownCObjectType(pub Dart_CObject_Type);

impl_from_to_pseudo_enums! {
    /// The type of typed data in a [`CObject`](crate::cobject::CObject).
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum TypedDataType from Dart_TypedData_Type {
        type Error = UnknownTypedDataType;
        ByteData = Dart_TypedData_kByteData,
        Int8 = Dart_TypedData_kInt8,
        Uint8 = Dart_TypedData_kUint8,
        Uint8Clamped = Dart_TypedData_kUint8Clamped,
        Int16 = Dart_TypedData_kInt16,
        Uint16 = Dart_TypedData_kUint16,
        Int32 = Dart_TypedData_kInt32,
        Uint32 = Dart_TypedData_kUint32,
        Int64 = Dart_TypedData_kInt64,
        Uint64 = Dart_TypedData_kUint64,
        Float32 = Dart_TypedData_kFloat32,
        Float64 = Dart_TypedData_kFloat64,
        Int32x4 = Dart_TypedData_kInt32x4,
        Float32x4 = Dart_TypedData_kFloat32x4,
        Float64x2 = Dart_TypedData_kFloat64x2,
    }
}

/// The [`CObjectType`] isn't known/supported by this library.
///
/// There are a few cases where a type is not supported:
///
/// - It was added in a newer Dart VM version.
/// - It's the `Dart_TypedData_kInvalid` type.
#[derive(Debug, Error)]
#[error("UnknownTypedDataType: {:?}", _0)]
pub struct UnknownTypedDataType(pub Dart_TypedData_Type);
