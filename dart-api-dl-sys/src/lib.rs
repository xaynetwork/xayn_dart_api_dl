//! Bindings for dart_api_dl.h
//!
//! The dart_api_dl library is for allowing code which is embedded/
//! loaded into dart to interact with the dart VM.
//!
//! This is for the case where rust is embedded/loaded into dart, not for
//! the case where dart is embedded into rust.
//!
//! A common use case for this would be a *flutter plugin* implemented
//! in dart which needs to send messages to a port.
//!
//! This library provides just the auto generated bindings and statically
//! links in the necessary C glue code **it's strongly recommended to
//! use the `dart-api-dl` library, which provides a still low-level but
//! slightly nicer to use interface**.
//!
//! # Supported Dart Versions
//!
//! Any dart vm with a dart dl api version >=2.0 and
//! <3.0 are supported. This means the min supported
//! dart version is 2.12. Known compatible versions
//! include 2.13, 2.14 and 2.15 (through 2.15 adds
//! an new CObject variant we do not yet support).
//!
//! # Dart Functions
//!
//! Except [`Dart_InitializeApiDL`] all functions are provided through
//! global variables containing function pointers which are set when
//! [`Dart_InitializeApiDL`] is called successfully.
//!
//! Accessing any of this global variable before [`Dart_InitializeApiDL`]
//! completed should be treated as unsound, even if you do null pointer
//! checks.
//!
//! ## Dart DL API Version Handling
//!
//! The dart DL API is separately versioned from dart. Calling
//! [`Dart_InitializeApiDL`] will fail if the major version doesn't
//! match. **It won't fail if the minor version doesn't match.**
//!
//! Using bindings with a lower minor version (e.g. 2.0) then
//! that of the dart vm (e.g. 2.1) is not a problem at all
//! and no special care must be taking in that case.
//!
//! But if using dart bindings with a higher minor version with
//! a dart vm having a lower minor version you need to consider
//! following:
//!
//! - Some function pointer might be null even after [`Dart_InitializeApiDL`]
//!   was called.
//!
//! - You must not use variants of [`Dart_CObject_Type`]/[`Dart_CObject`] which
//!   didn't exist in the dart VM's API version.
//!
//! The const [`DART_API_DL_MAJOR_VERSION`] and [`DART_API_DL_MINOR_VERSION`]
//! represent the version of this bindings.
//!
//! The version of the dart vm's DL API **can not be look up, the functionality
//! is missing.** Currently we are at DL API version 2.0 so it doesn't matter.
//!
// FIXME: But Dart 2.15 bumps it to 2.1 in a non-detectable way.
//        If it's not fixed by dart we will fix it by accessing ABI stable
//        implementation details, but I hope we don't need to.
//        Also versions are accessible from dart, so higher level bindings
//        can handle it, somewhat, not very nicely.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// This is triggered by auto generated alignment *tests*, which unsafely
// turn a nullptr into a reference.
#![allow(deref_nullptr)]

include!(concat!(env!("OUT_DIR"), "/dart_api_dl_bindings.rs"));


#[cfg(test)]
mod tests {
    #![deny(deref_nullptr)]
    use static_assertions::assert_type_eq_all;

    use super::*;

    #[test]
    fn dart_port_is_dart_port_dl() {
        assert_type_eq_all!(Dart_Port, Dart_Port_DL);
    }
}