use std::{env, fs::{self, remove_dir_all}, path::{Path, PathBuf}, process::Command};

use bindgen::EnumVariation;
use semver::Version;

static DL_ENABLED_FUNCTIONS: &[&str] = &[
  "Dart_InitializeApiDL",
];

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
    let dart_src_ws = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("dart-src");

    if !dart_src_ws.exists() {
        create_dir(&dart_src_ws);
    }

    if !dart_src_ws.is_dir() {
        panic!("Expected a directory: {}", dart_src_ws.display());
    }

    print!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
    let version: Version = env::var("CARGO_PKG_VERSION").unwrap().parse().unwrap();
    let dart_src_dir = dart_src_ws.join(version.build.as_str());
    if !dart_src_dir.exists() {
        download_dart_src(version.build.as_str(), &dart_src_dir);
    }

    let dl_header_path = dart_src_dir.join("dart_api_dl.h");
    let dl_version_header_path = dart_src_dir.join("dart_version.h");

    let mut builder = bindgen::Builder::default()
        .header(dl_header_path.to_str().expect("non-utf8 path"))
        .header(dl_version_header_path.to_str().expect("non-utf8 path"))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .default_enum_style(EnumVariation::NewType { is_bitfield: false});

    for function in DL_ENABLED_FUNCTIONS {
        builder = builder.allowlist_function(function);
    }

    for r#type in DL_ENABLED_TYPES {
        builder = builder.allowlist_type(r#type);
    }

    for var in DL_ENABLED_VARS {
        builder = builder.allowlist_var(var);
    }

    let bindings = builder.generate()
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
