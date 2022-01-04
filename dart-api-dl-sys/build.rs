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

use std::{env, path::PathBuf};

use bindgen::EnumVariation;

static DL_ENABLED_FUNCTIONS: &[&str] = &["Dart_InitializeApiDL"];

static DL_ENABLED_TYPES: &[&str] = &[
    "Dart_.+_DL",
    "Dart_CObject",
    "Dart_Handle",
    "Dart_PersistentHandle",
    "Dart_WeakPersistentHandle",
    "Dart_HandleFinalizer",
    "Dart_FinalizableHandle",
    "Dart_CObject_Type",
    "Dart_TypedData_Type",
];

static DL_ENABLED_VARS: &[&str] = &[
    "Dart_.+_DL",
    "DART_API_DL_MAJOR_VERSION",
    "DART_API_DL_MINOR_VERSION",
];

fn main() {
    print!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    let dart_src_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .join("dart-src");

    let dl_header_path = dart_src_dir.join("dart_api_dl.h");
    let dl_version_header_path = dart_src_dir.join("dart_version.h");

    let mut builder = bindgen::Builder::default()
        .header(dl_header_path.to_str().expect("non-utf8 path"))
        .header(dl_version_header_path.to_str().expect("non-utf8 path"))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .default_enum_style(EnumVariation::NewType { is_bitfield: false });

    for function in DL_ENABLED_FUNCTIONS {
        builder = builder.allowlist_function(function);
    }

    for r#type in DL_ENABLED_TYPES {
        builder = builder.allowlist_type(r#type);
    }

    for var in DL_ENABLED_VARS {
        builder = builder.allowlist_var(var);
    }

    let bindings = builder
        .generate()
        .expect("Failed to generate dart_api_dl binding");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("dart_api_dl_bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Failed to write dat_api_dl bindings.");

    let dl_glue_path = dart_src_dir.join("dart_api_dl.c");
    cc::Build::new()
        .file(dl_glue_path)
        .include(dart_src_dir)
        .compile("dart_api_dl");
}
