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
pub type ExternalTypedData = _Dart_CObject__bindgen_ty_1__bindgen_ty_5;

/// Wrapper around a `Dart_CObject` which can be read, but which we do not own.
///
/// As such we can't deallocate anything in it and should in general not modify it.
///
// Transparent repr is very important as we will "unsafe" cast between the dart type
// and our new-type which we use to attach methods and safety to the dart type.
#[repr(transparent)]

pub struct ExternCObject {
    obj: Dart_CObject,
}

impl ExternCObject {
    ///
    /// # Safety
    ///
    /// 1. the pointer must point to a valid Dat_CObject
    /// 2. the Dat_CObject must be sound/consistent, e.g.
    ///    the type and set data match and are sound. This
    ///    moves the unsafety of the various `as_*` methods
    ///    into this method.
    pub unsafe fn with_pointer<R>(
        ptr: *mut Dart_CObject,
        func: impl FnOnce(&mut ExternCObject) -> R,
    ) -> R {
        let temp_ref = &mut *(ptr as *mut ExternCObject);
        func(temp_ref)
    }

    ///
    /// # Safety
    ///
    /// If you use unsafe code to modify the underlying object
    /// you MUST make sure it still is sound.
    pub fn as_ptr_mut(&mut self) -> *mut Dart_CObject {
        &mut self.obj
    }

    /// Set type to null, doesn't run any drop and as such might leak memory.
    pub(crate) fn set_to_null(&mut self) {
        self.obj.type_ = Dart_CObject_Type::Dart_CObject_kNull;
    }

    pub fn r#type(&self) -> Result<CObjectType, UnknownCObjectType> {
        self.obj.type_.try_into()
    }

    pub fn as_null(&self, rt: DartRuntime) -> Option<()> {
        if let Ok(CObjectRef::Null) = self.value_ref(rt) {
            Some(())
        } else {
            None
        }
    }

    pub fn as_bool(&self, rt: DartRuntime) -> Option<bool> {
        if let Ok(CObjectRef::Bool(b)) = self.value_ref(rt) {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_int32(&self, rt: DartRuntime) -> Option<i32> {
        if let Ok(CObjectRef::Int32(v)) = self.value_ref(rt) {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_int64(&self, rt: DartRuntime) -> Option<i64> {
        if let Ok(CObjectRef::Int64(v)) = self.value_ref(rt) {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_int(&self, rt: DartRuntime) -> Option<i64> {
        if let Some(v) = self.as_int32(rt) {
            Some(v as i64)
        } else {
            self.as_int64(rt)
        }
    }

    pub fn as_double(&self, rt: DartRuntime) -> Option<f64> {
        if let Ok(CObjectRef::Double(d)) = self.value_ref(rt) {
            Some(d)
        } else {
            None
        }
    }

    pub fn as_string(&self, rt: DartRuntime) -> Option<&str> {
        if let Ok(CObjectRef::String(s)) = self.value_ref(rt) {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_array(&self, rt: DartRuntime) -> Option<&[&ExternCObject]> {
        if let Ok(CObjectRef::Array(array)) = self.value_ref(rt) {
            Some(array)
        } else {
            None
        }
    }

    pub fn as_typed_data(
        &self,
        rt: DartRuntime,
    ) -> Option<(Result<TypedDataRef, UnknownTypedDataType>, bool)> {
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

    pub fn as_send_port(&self, rt: DartRuntime) -> Option<Option<SendPort>> {
        if let Ok(CObjectRef::SendPort(port)) = self.value_ref(rt) {
            Some(port)
        } else {
            None
        }
    }

    pub fn as_capability(&self, rt: DartRuntime) -> Option<Capability> {
        if let Ok(CObjectRef::Capability(cap)) = self.value_ref(rt) {
            Some(cap)
        } else {
            None
        }
    }

    pub fn typed_data_type(&self) -> Option<Result<TypedDataType, UnknownTypedDataType>> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kTypedData
            || self.obj.type_ == Dart_CObject_Type::Dart_CObject_kExternalTypedData)
            .then(|| {
                // Safe:
                // - if CObject is sound (which is required) the type check is enough
                // - Like done by dart-lang/sdk, `as_external_typed_data` is made so that
                //   it starts with the same layout as `as_typed_data`.
                unsafe { self.obj.value.as_typed_data.type_ }.try_into()
            })
    }

    /// If the type is known returns a enums with a type specific reference to the data.
    ///
    /// Copy types are provided as copy instead of as
    pub fn value_ref(&self, rt: DartRuntime) -> Result<CObjectRef, UnknownCObjectType> {
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
            CObjectType::Double => Ok(Double(unsafe { self.obj.value.as_double })),
            CObjectType::String => {
                // Safe:
                // - CObject is sound
                // - we checked the type
                Ok(String(unsafe {
                    let c_str = CStr::from_ptr(self.obj.value.as_string);
                    // Unwrap: Strings in CObject are always UTF-8
                    std::str::from_utf8(c_str.to_bytes()).unwrap()
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
                    let ptr = ar.values as *const &ExternCObject;
                    slice::from_raw_parts(ptr, ar.length.try_into().unwrap())
                }))
            }
            CObjectType::TypedData | CObjectType::ExternalTypedData => {
                //Unwrap: We know there is a typed data type.
                let data = self.typed_data_type().unwrap().map(|data_type| {
                    // Safe:
                    // - CObject is sound
                    // - we checked the type
                    unsafe {
                        let data = &self.obj.value.as_typed_data;
                        TypedDataRef::from_raw(
                            data_type,
                            data.values as *const u8,
                            data.length.try_into().unwrap(),
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

impl Debug for ExternCObject {
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

#[derive(Debug)]
pub enum CObjectRef<'a> {
    Null,
    Bool(bool),
    Int32(i32),
    Int64(i64),
    Double(f64),
    String(&'a str),
    Array(&'a [&'a ExternCObject]),
    TypedData {
        data: Result<TypedDataRef<'a>, UnknownTypedDataType>,
        external_typed: bool,
    },
    SendPort(Option<SendPort>),
    Capability(Capability),
}

#[derive(Debug, Clone, Copy)]
pub enum TypedDataRef<'a> {
    ByteData(&'a [u8]),
    Int8(&'a [i8]),
    Uint8(&'a [u8]),
    Uint8Clamped(&'a [u8]),
    Int16(&'a [i16]),
    Uint16(&'a [u16]),
    Int32(&'a [i32]),
    Uint32(&'a [u32]),
    Int64(&'a [i64]),
    Uint64(&'a [u64]),
    Float32(&'a [f32]),
    Float64(&'a [f64]),
    Int32x4(&'a [[i32; 4]]),
    Float32x4(&'a [[f32; 4]]),
    Float64x2(&'a [[f64; 2]]),
}

impl TypedDataRef<'_> {
    unsafe fn from_raw(data_type: TypedDataType, data: *const u8, len: usize) -> Self {
        use self::TypedDataRef::*;
        use std::slice::from_raw_parts;
        match data_type {
            TypedDataType::ByteData => ByteData(from_raw_parts(data, len)),
            TypedDataType::Int8 => Int8(from_raw_parts(data as *const i8, len)),
            TypedDataType::Uint8 => Uint8(from_raw_parts(data, len)),
            TypedDataType::Uint8Clamped => Uint8Clamped(from_raw_parts(data, len)),
            TypedDataType::Int16 => Int16(from_raw_parts(data as *const i16, len)),
            TypedDataType::Uint16 => Uint16(from_raw_parts(data as *const u16, len)),
            TypedDataType::Int32 => Int32(from_raw_parts(data as *const i32, len)),
            TypedDataType::Uint32 => Uint32(from_raw_parts(data as *const u32, len)),
            TypedDataType::Int64 => Int64(from_raw_parts(data as *const i64, len)),
            TypedDataType::Uint64 => Uint64(from_raw_parts(data as *const u64, len)),
            TypedDataType::Float32 => Float32(from_raw_parts(data as *const f32, len)),
            TypedDataType::Float64 => Float64(from_raw_parts(data as *const f64, len)),
            TypedDataType::Int32x4 => Int32x4(from_raw_parts(data as *const [i32; 4], len)),
            TypedDataType::Float32x4 => Float32x4(from_raw_parts(data as *const [f32; 4], len)),
            TypedDataType::Float64x2 => Float64x2(from_raw_parts(data as *const [f64; 2], len)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypedData {
    ByteData(Box<[u8]>),
    Int8(Vec<i8>),
    Uint8(Vec<u8>),
    Uint8Clamped(Vec<u8>),
    Int16(Vec<i16>),
    Uint16(Vec<u16>),
    Int32(Vec<i32>),
    Uint32(Vec<u32>),
    Int64(Vec<i64>),
    Uint64(Vec<u64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    Int32x4(Vec<[i32; 4]>),
    Float32x4(Vec<[f32; 4]>),
    Float64x2(Vec<[f64; 2]>),
}

impl TypedData {
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
                let ptr = data.as_mut_ptr() as *mut u8;
                let length = data.len().try_into().unwrap();
                let peer = Box::into_raw(Box::new(data)) as *mut c_void;

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
                let ptr = data.as_mut_ptr() as *mut u8;
                let length = data.len().try_into().unwrap();
                let peer = Box::into_raw(Box::new(data)) as *mut c_void;

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
    drop(Box::from_raw(peer as *mut T));
}

/// Wrapper around a `Dart_CObject` which is owned by rust.
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedCObject(ExternCObject);

impl OwnedCObject {
    //not meant to be public, just a helper to reduce code duplication
    fn wrap_raw(obj: Dart_CObject) -> Self {
        Self(ExternCObject { obj })
    }

    pub fn null() -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kNull,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: false },
        })
    }

    pub fn bool(val: bool) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kBool,
            value: _Dart_CObject__bindgen_ty_1 { as_bool: val },
        })
    }

    pub fn int32(val: i32) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt32,
            value: _Dart_CObject__bindgen_ty_1 { as_int32: val },
        })
    }

    pub fn int64(val: i64) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt64,
            value: _Dart_CObject__bindgen_ty_1 { as_int64: val },
        })
    }

    pub fn double(val: f64) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kDouble,
            value: _Dart_CObject__bindgen_ty_1 { as_double: val },
        })
    }

    pub fn string(val: impl AsRef<str>) -> Result<Self, NulError> {
        let val = CString::new(val.as_ref())?;
        Ok(Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 {
                as_string: val.into_raw(),
            },
        }))
    }

    /// Like string, but cut's of when encountering a `'\0'`.
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

    /// Create a CObject representing a send port.
    pub fn send_port(port: SendPort) -> Self {
        let (id, origin_id) = port.as_raw();
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kSendPort,
            value: _Dart_CObject__bindgen_ty_1 {
                as_send_port: _Dart_CObject__bindgen_ty_1__bindgen_ty_1 { id, origin_id },
            },
        })
    }

    pub fn capability(id: i64) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kCapability,
            value: _Dart_CObject__bindgen_ty_1 {
                as_capability: _Dart_CObject__bindgen_ty_1__bindgen_ty_2 { id },
            },
        })
    }

    pub fn array(array: Vec<Box<OwnedCObject>>) -> Self {
        let bs = array.into_boxed_slice();
        let len = bs.len().try_into().unwrap();
        // SAFE: as CObject is repr(transparent) and box and *mut have same layout
        let ptr = Box::into_raw(bs) as *mut *mut Dart_CObject;
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

    pub fn typed_data(data: TypedData) -> Self {
        // TODO If we can have a lifetime on cobjects this can
        //      make sense to implement by accepting a TypedDataRef
        Self::external_typed_data(data)
    }

    pub fn external_typed_data<CET>(data: CET) -> Self
    where
        CET: CustomExternalTyped,
    {
        return Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kExternalTypedData,
            value: _Dart_CObject__bindgen_ty_1 {
                //Safe: due to the unsafe contract on CustomExternalTyped
                as_external_typed_data: data.into_external_typed_data(),
            },
        });
    }
}

impl Deref for OwnedCObject {
    type Target = ExternCObject;

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
                let len = self.obj.value.as_array.length as usize;
                let ptr = self.obj.value.as_array.values;
                Vec::from_raw_parts(ptr, len, len)
            }),
            Dart_CObject_Type::Dart_CObject_kTypedData => {
                // we don't create this currently, so we can't be in a
                // situation where we need to drop it.
                panic!("unsupported `OwnedCObject` format");
            }
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
                        data as *mut c_void,
                        peer,
                    );
                }
            }
            Dart_CObject_Type::Dart_CObject_kNumberOfTypes
            | Dart_CObject_Type::Dart_CObject_kUnsupported
            | _ => {
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
/// use in a CObject.
///
pub unsafe trait CustomExternalTyped {
    /// This should only be called by the `OwnedCObject` type.
    ///
    /// Directly dropping the return type of this function will
    /// leak the resources of this instance. Through `OwnedCObject`
    /// will make sure that this doesn't happen.
    fn into_external_typed_data(self) -> ExternalTypedData;
}
