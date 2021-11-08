//! A small rust script for updating this library.
//!
//! It could be a single file if I would require
//! e.g. `rust-script` but I don't want to require
//! such a dependency when there is little benefit
//! in it.

use std::{fs::{self, remove_dir_all}, path::{Path, PathBuf}, process::Command};
fn main() {
    let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dart_src_ws = Path::

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
}



fn download_dart_src(dart_version: &str, out_dir: &Path) {
    eprintln!("Downloading dart version: {:?}", dart_version);
    let git_out_dir = temp_dir();
    let ec = Command::new("git")
        .args(&["clone", "--depth", "1", "--branch"])
        .arg(dart_version)
        .args(&["--", "https://github.com/dart-lang/sdk.git"])
        .arg(git_out_dir.display().to_string())
        .output()
        .unwrap();

    if !ec.status.success() {
        panic!(
            "failed to fetch dart source: {}",
            String::from_utf8_lossy(&ec.stderr)
        );
    }

    create_dir(&out_dir);
    copy_all_in(&git_out_dir.join("runtime/include"), out_dir, &["c", "h"]);
    copy_file(&git_out_dir.join("LICENSE"), &out_dir.join("LICENSE"));

    remove_dir_all(&git_out_dir).unwrap_or_else(|e| {
        panic!(
            "Failed to cleanup temp dir: {}\n{}",
            git_out_dir.display(),
            e
        )
    });
}

fn copy_all_in(target_dir: &Path, out_dir: &Path, endings: &[&str]) {
    for dir_entry in target_dir
        .read_dir()
        .unwrap_or_else(|e| panic!("Copying files failed: {}\n{}", target_dir.display(), e))
    {
        let dir_entry = dir_entry.unwrap();
        let f_type = dir_entry.file_type().unwrap();
        let from_path = &dir_entry.path();
        let to_path = &out_dir.join(dir_entry.file_name());
        if f_type.is_dir() {
            create_dir(to_path);
            copy_all_in(from_path, to_path, endings);
        } else if f_type.is_file() && from_path.extension().map(|stem| {
            let stem = stem.to_str().unwrap();
            endings.contains(&stem)
        }).unwrap_or(false) {

            copy_file(from_path, to_path);
        }
    }
}

fn temp_dir() -> PathBuf {
    let out = Command::new("mktemp")
        .args(&["-d", "-t", "dart-api-dl-codegen.XXXX"])
        .output()
        .unwrap();
    if !out.status.success() {
        panic!("Creating temp dir failed.");
    }
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
}

fn create_dir(name: &Path) {
    fs::create_dir(name)
        .unwrap_or_else(|e| panic!("Failed to create dir: {}\n{}", name.display(), e));
}

fn copy_file(from: &Path, to: &Path) {
    fs::copy(from, to)
        .unwrap_or_else(|e| panic!("Failed to copy file from {} to {}\n{}", from.display(), to.display(), e));
}