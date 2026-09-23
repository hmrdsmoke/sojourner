// ── Add your header block above ──

//! Build script. One job: finish a link that espeak-rs-sys leaves undone.
//!
//! espeak-rs-sys compiles espeak-ng into a static library. espeak-ng's
//! wave generator calls into sonic (a small speed-change library, Apache-2.0)
//! and its CMake build, finding the system's libsonic, records that
//! dependency for itself — but a static library carries no link
//! information, and espeak-rs-sys's own build script only tells Cargo about
//! the libraries it produced. The result is a final link with seven
//! undefined `sonic*` symbols. This script adds the missing line.
//!
//! sonic comes from `libsonic-dev` (Debian/Ubuntu/Pop!_OS). It is linked
//! statically when its archive can be found, so the finished binary needs
//! no libsonic.so on the machine it runs on; otherwise it is linked as a
//! shared library. Sojourner never calls sonic; it is there because
//! espeak-ng was built with it. See assets/SOURCES.md, section 4.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Ask the C compiler where libsonic.a is. gcc and clang both answer
    // `-print-file-name` with the full path when the file is in their
    // library search path, and with the bare name when it isn't.
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let archive = Command::new(&cc)
        .arg("-print-file-name=libsonic.a")
        .output()
        .ok()
        .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
        .filter(|path| path.is_absolute() && path.is_file());

    match archive {
        Some(path) => {
            println!("cargo:rustc-link-search=native={}", path.parent().unwrap().display());
            println!("cargo:rustc-link-lib=static=sonic");
        }
        None => {
            println!("cargo:warning=libsonic.a not found (is libsonic-dev installed?); linking sonic as a shared library");
            println!("cargo:rustc-link-lib=sonic");
        }
    }
}
