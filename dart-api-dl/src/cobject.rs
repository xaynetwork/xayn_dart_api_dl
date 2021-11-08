use std::{ffi::{CStr, CString, c_void}, io::Read, ops::Deref, slice};

use dart_api_dl_sys::{Capability, Dart_CObject, Dart_CObject_Type, Dart_Port, Dart_Port_DL, Dart_TypedData_Type, ExternalTypedData, SendPorts, _Dart_CObject__bindgen_ty_1, _Dart_CObject__bindgen_ty_1__bindgen_ty_1, _Dart_CObject__bindgen_ty_1__bindgen_ty_2, _Dart_CObject__bindgen_ty_1__bindgen_ty_3, _Dart_CObject__bindgen_ty_1__bindgen_ty_4, _Dart_CObject__bindgen_ty_1__bindgen_ty_5};

/// Wrapper around a `Dart_CObject` which can be read, but which we do not own.
///
/// As such we can't deallocate anything in it and should in general not modify it.
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
        func: impl FnOnce(&ExternCObject) -> R,
    ) -> R {
        let temp_ref = &*(ptr as *mut ExternCObject);
        func(temp_ref)
    }

    pub fn r#type(&self) -> Dart_CObject_Type {
        self.obj.type_
    }

    pub fn is_null(&self) -> bool {
        self.obj.type_ == Dart_CObject_Type::Dart_CObject_kBool
    }

    pub fn as_bool(&self) -> Option<bool> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kBool).then(
            || self.obj.value.as_bool
        )
    }

    pub fn as_i32(&self) -> Option<i32> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kInt32).then(
            || self.obj.value.as_int32
        )
    }

    pub fn as_i64(&self) -> Option<i64> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kInt64).then(
            || self.obj.value.as_int64
        )
    }

    pub fn as_double(&self) -> Option<f64> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kDouble).then(
            || self.obj.value.as_double
        )
    }

    pub fn as_str(&self) -> Option<&str> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kString).then(
            //Safe:
            // 1. safety guarantees of the constructor
            // 2. dart strings in CObjects are guaranteed to be utf-8
            || unsafe {  std::str::from_utf8_unchecked(CStr::from_ptr(self.obj.value.as_string).to_bytes()) }
        )
    }

    pub fn as_send_port(&self) -> Option<SendPorts> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kSendPort).then(
            || self.obj.value.as_send_port
        )
    }

    pub fn as_capability(&self) -> Option<Capability> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kCapability).then(
            || self.obj.value.as_capability.id
        )
    }

    pub fn as_slice(&self) -> Option<&[&ExternalTypedData]> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kArray).then(
            || {
                let ar = &self.obj.value.as_array;
                // Safe:
                // 1. the unsafe contract on the constructor
                // 2. ExternalTypedData being repr(transparent)
                // 3. *const/*mut/& all have the same representation
                // 4. mut => const is a ok
                unsafe {
                    let ptr = ar.values as *const &ExternalTypedData;
                    slice::from_raw_parts(ptr, ar.length.try_into().unwrap())
                }
            }
        )
    }

    pub fn typed_data_type(&self) -> Option<Dart_TypedData_Type> {
        (self.obj.type_ == Dart_CObject_Type::Dart_CObject_kTypedData ||
            self.obj.type_ == Dart_CObject_Type::Dart_CObject_kExternalTypedData).then(
                || {
                   // Like done by dart-lang/sdk, `as_external_typed_data` is made so that
                   // it starts with the same layout as `as_typed_data`.
                   self.obj.value.as_typed_data.type_
                }
            )
    }

    /// Returns a slice of typed data.
    ///
    /// This works both with `kTYpedData` and `kExternalTypedData`. Dart does
    /// handle the finalization (dropping) of external typed data fully once
    /// you did send it. I.e. we only care about dropping it when we created
    /// it but didn't use it. As such no `as_external_typed_data` is needed.
    ///
    //FIXME add support for custom external typed data which adds special meaning
    pub fn as_typed_data_of<T>(&self) -> Option<&[T]>
    where
        T: TypedData
    {
        let typed_data_type = self.typed_data_type()?;

        //FIXME support Bytes/Uint8Clamped as Vec<u8>
        (T::Type == typed_data_type).then(
            || {
                let data = &self.obj.value.as_typed_data;
                //Safe:
                // 1. unsafe contract of constructor requires the Dart_CObject to be sound
                // 2. external and normal typed data are the same (incl. same ABI) when reading from
                //    them. At least for the Dart DL API major version 2 this is guaranteed to not
                //    change.
                unsafe {
                    slice::from_raw_parts(data.values as *mut T, data.length.try_into().unwrap())
                }
            }
        )
    }
}

/// Wrapper around a `Dart_CObject` which is owned by rust.
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

    pub fn i32(val: i32) -> Self {
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kInt32,
            value: _Dart_CObject__bindgen_ty_1 { as_int32: val },
        })
    }

    pub fn i64(val: i64) -> Self {
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

    pub fn string(val: CString) -> Self {
        let val = val.into_raw();
        Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kString,
            value: _Dart_CObject__bindgen_ty_1 { as_string: val },
        })
    }

    pub fn send_port(id: Dart_Port_DL, origin_id: Dart_Port_DL) -> Self {
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

    pub fn array(mut array: Vec<Box<OwnedCObject>>) -> Self {
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

    pub fn typed_data<T>(data: Vec<T>) -> Self
    where
        T: TypedData,
    {
        // given the current design there is little reason not to use
        // external_typed data, through we might change this in the future
        Self::external_typed_data(data)
    }

    pub fn external_typed_data<T>(mut data: Vec<T>) -> Self
    where
        T: TypedData,
    {
        let len = data.len().try_into().unwrap();
        let ptr = data.as_mut_ptr() as *mut u8;
        let peer = Box::into_raw(Box::new(data)) as *mut c_void;
        return Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kExternalTypedData,
            value: _Dart_CObject__bindgen_ty_1 {
                as_external_typed_data: _Dart_CObject__bindgen_ty_1__bindgen_ty_5 {
                    type_: T::Type,
                    length: len,
                    data: ptr,
                    peer: peer,
                    callback: Some(drop_vec::<T>),
                },
            },
        });

        unsafe extern "C" fn drop_vec<T>(_data: *mut c_void, peer: *mut c_void) {
            drop(Box::from_raw(peer as *mut Vec<T>));
        }
    }

    pub fn custom_external_type<CET>(mut data: CET) -> Self
    where
        CET: CustomExternalTyped,
    {
        return Self::wrap_raw(Dart_CObject {
            type_: Dart_CObject_Type::Dart_CObject_kExternalTypedData,
            value: _Dart_CObject__bindgen_ty_1 {
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
                let etd = &self.obj.value.as_external_typed_data;
                let data = etd.data;
                let peer = etd.peer;
                let callback = etd.callback;
                self.obj.type_ = Dart_CObject_Type::Dart_CObject_kNull;
                (callback.expect("unexpected null pointer callback"))(data as *mut c_void, peer);
            }
            Dart_CObject_Type::Dart_CObject_kNumberOfTypes
            | Dart_CObject_Type::Dart_CObject_kUnsupported => {
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

/// Helper trait for implementing external typed data for Vec<primitives>.
///
/// # Safety
///
/// 1. must not implement Drop
/// 2. must not include any padding inside of it, reading and writing
///    the type cast to a byte pointer must be valid.
/// 3. align and size must allow creating arrays of the type without padding
///    (i.e. the size must be a multiple of the alignment)
unsafe trait TypedData: Copy {
    const Type: Dart_TypedData_Type;
}

macro_rules! impl_typed_data {
    ($($name:ty, $kname:ident);*) => ($(
        unsafe impl TypedData for $name {
            const Type: Dart_TypedData_Type = Dart_TypedData_Type::$kname;
        }
    )*);
}

//TODO support special variants of u8:
// - Bytes, fixed size grate in combination with a buffer reuse (mem-pool)
//   to have a high data through put. As it's fixed size the len can be
//   prefixed to the content avoiding boxing
// - Uint8Clamped, basically just Uint8 but with a hint for dart to treat it
//   differently when assigning out of bounds integers
impl_typed_data!(u8, Dart_TypedData_kUint8);
impl_typed_data!(i8, Dart_TypedData_kInt8);
impl_typed_data!(u16, Dart_TypedData_kUint16);
impl_typed_data!(i16, Dart_TypedData_kInt16);
impl_typed_data!(u32, Dart_TypedData_kUint32);
impl_typed_data!(i32, Dart_TypedData_kInt32);
impl_typed_data!(u64, Dart_TypedData_kUint64);
impl_typed_data!(i64, Dart_TypedData_kInt64);
impl_typed_data!(f32, Dart_TypedData_kFloat32);
impl_typed_data!(f64, Dart_TypedData_kFloat64);
impl_typed_data!([f32; 4], Dart_TypedData_kFloat32x4);
impl_typed_data!([f64; 2], Dart_TypedData_kFloat64x2);
impl_typed_data!([i32; 4], Dart_TypedData_kFloat32x4);

/// Hook to allow using custom external typed data.
///
/// # Safety
///
/// The returned external typed data must be sound to
/// use in a CObject.
///
unsafe trait CustomExternalTyped {
    /// This should only be called by the `OwnedCObject` type.
    ///
    /// Directly dropping the return type of this function will
    /// leak the resources of this instance. Through `OwnedCObject`
    /// will make sure that this doesn't happen.
    fn into_external_typed_data(self) -> ExternalTypedData;
}


