// ── Add your header block above ──

//! Where the voice and the runtime library live.
//!
//! The voice model is over a hundred megabytes, so it is not in the
//! repository and not compiled into the binary; it is a build-time source,
//! pinned by hash in assets/SOURCES.md. Likewise ONNX Runtime, the library
//! that runs the model, is loaded at startup from a path rather than linked
//! in, so a Store build never has to download anything. Both are looked for
//! in this order:
//!
//! 1. Explicit: `$SOJOURNER_VOICE_DIR` and `$SOJOURNER_ORT_LIB`.
//! 2. Inside the Flatpak: `/app/share/sojourner/voices` and `/app/lib`.
//! 3. In a source checkout: `assets/voices` and `assets/onnxruntime/*/lib`
//!    (the path is compiled in from the build, so this only ever works for
//!    a `cargo run`).
//! 4. Per user: `$XDG_DATA_HOME/sojourner/voices`, else
//!    `~/.local/share/sojourner/voices`.
//!
//! espeak-ng's phoneme data is a third file set. In a source build
//! espeak-ng finds it through the path compiled in at build time (under
//! `target/`), which is enough for `cargo run` and `cargo test`; a packaged
//! build ships the `espeak-ng-data` directory and `find_espeak_data` points
//! the engine at it.

use std::path::{Path, PathBuf};

use super::Error;

/// The voice Sojourner ships: LJ Speech, high quality. See SOURCES.md.
pub const VOICE_STEM: &str = "en_US-ljspeech-high";

/// The directories a voice is looked for in, in order of preference.
fn voice_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(dir) = std::env::var("SOJOURNER_VOICE_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    dirs.push(PathBuf::from("/app/share/sojourner/voices"));
    dirs.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/voices"));
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        dirs.push(Path::new(&xdg).join("sojourner/voices"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/sojourner/voices"));
    }
    dirs
}

/// The model and its config, if the voice can be found.
pub fn find_voice() -> Result<(PathBuf, PathBuf), Error> {
    let dirs = voice_dirs();
    for dir in &dirs {
        let model = dir.join(format!("{VOICE_STEM}.onnx"));
        let config = dir.join(format!("{VOICE_STEM}.onnx.json"));
        if model.is_file() && config.is_file() {
            return Ok((model, config));
        }
    }
    Err(Error::Missing(format!(
        "voice {VOICE_STEM}.onnx (+ .onnx.json); looked in {}",
        dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
    )))
}

/// The ONNX Runtime shared library, if it can be found.
pub fn find_onnxruntime() -> Result<PathBuf, Error> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(lib) = std::env::var("SOJOURNER_ORT_LIB") {
        candidates.push(PathBuf::from(lib));
    }
    candidates.push(PathBuf::from("/app/lib/libonnxruntime.so"));
    // assets/onnxruntime/<release>/lib/libonnxruntime.so, whatever the
    // release directory is called.
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/onnxruntime");
    if let Ok(entries) = std::fs::read_dir(&dev) {
        for entry in entries.flatten() {
            candidates.push(entry.path().join("lib/libonnxruntime.so"));
        }
    }
    for path in &candidates {
        if path.is_file() {
            return Ok(path.clone());
        }
    }
    Err(Error::Missing(format!(
        "libonnxruntime.so; looked at {}",
        candidates.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
    )))
}

/// espeak-ng's data directory, when Sojourner has to say where it is:
/// `$SOJOURNER_ESPEAK_DATA`, else the Flatpak's copy. `None` means let
/// espeak-ng use the path compiled in at build time, which is right for a
/// source build.
pub fn find_espeak_data() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SOJOURNER_ESPEAK_DATA") {
        return Some(PathBuf::from(dir));
    }
    let flatpak = PathBuf::from("/app/share/sojourner/espeak-ng-data");
    flatpak.is_dir().then_some(flatpak)
}

/// Point the runtime at the library. Must happen once, before any voice is
/// opened. Harmless to call again.
pub fn init_runtime() -> Result<PathBuf, Error> {
    let lib = find_onnxruntime()?;
    ort::init_from(&lib)
        .map_err(|e| Error::Engine(format!("loading {}: {e}", lib.display())))?
        .commit();
    Ok(lib)
}
