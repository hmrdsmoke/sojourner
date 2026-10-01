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
//! 4. Installed: per user under `$XDG_DATA_HOME/sojourner` (else
//!    `~/.local/share/sojourner`) — `voices/` and `lib/libonnxruntime.so` —
//!    and system-wide under `/usr/local` and `/usr`: `share/sojourner/voices`
//!    and `lib/sojourner/libonnxruntime.so`. `tools/install.sh` lays the
//!    files out this way.
//!
//! espeak-ng's phoneme data is a third file set. In a source build
//! espeak-ng finds it through the path compiled in at build time (under
//! `target/`), which is enough for `cargo run` and `cargo test`; an
//! installed or packaged build ships the `espeak-ng-data` directory beside
//! the voices and `find_espeak_data` points the engine at it.

use std::path::{Path, PathBuf};

use super::Error;

/// The voice Sojourner ships: LJ Speech, high quality. See SOURCES.md.
pub const VOICE_STEM: &str = "en_US-ljspeech-high";

/// The user's own data directory for Sojourner: `$XDG_DATA_HOME/sojourner`,
/// else `~/.local/share/sojourner`.
fn user_data_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Some(Path::new(&xdg).join("sojourner"));
    }
    std::env::var("HOME").ok().map(|home| Path::new(&home).join(".local/share/sojourner"))
}

/// The system-wide data directories an installed Sojourner may have put
/// its files in.
const SYSTEM_SHARE: [&str; 2] = ["/usr/local/share/sojourner", "/usr/share/sojourner"];
const SYSTEM_LIB: [&str; 2] = ["/usr/local/lib/sojourner", "/usr/lib/sojourner"];

/// The directories a voice is looked for in, in order of preference.
fn voice_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(dir) = std::env::var("SOJOURNER_VOICE_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    dirs.push(PathBuf::from("/app/share/sojourner/voices"));
    dirs.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/voices"));
    if let Some(user) = user_data_dir() {
        dirs.push(user.join("voices"));
    }
    dirs.extend(SYSTEM_SHARE.iter().map(|d| Path::new(d).join("voices")));
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
    if let Some(user) = user_data_dir() {
        candidates.push(user.join("lib/libonnxruntime.so"));
    }
    candidates.extend(SYSTEM_LIB.iter().map(|d| Path::new(d).join("libonnxruntime.so")));
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
/// `$SOJOURNER_ESPEAK_DATA`, else the Flatpak's copy, else an installed
/// copy beside the voices. `None` means let espeak-ng use the path
/// compiled in at build time, which is right for a source build.
pub fn find_espeak_data() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SOJOURNER_ESPEAK_DATA") {
        return Some(PathBuf::from(dir));
    }
    let mut dirs = vec![PathBuf::from("/app/share/sojourner")];
    dirs.extend(user_data_dir());
    dirs.extend(SYSTEM_SHARE.iter().map(PathBuf::from));
    dirs.into_iter().map(|d| d.join("espeak-ng-data")).find(|d| d.is_dir())
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
