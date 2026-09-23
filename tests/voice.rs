// ── Add your header block above ──

//! Proof that the voice speaks: with the voice files and ONNX Runtime in
//! place (see src/voice/files.rs for where), synthesize one line of Psalm 23
//! and check that real audio came out. The result is written to
//! target/voice-check.wav so a person can listen to it.
//!
//! Skips, loudly, when the files aren't present: they are not in the
//! repository.

use sojourner::voice::{files, piper::PiperVoice, Voice};

#[test]
fn the_voice_speaks() {
    let (model, config) = match files::find_voice() {
        Ok(found) => found,
        Err(e) => {
            eprintln!("SKIPPED — {e}");
            return;
        }
    };
    let runtime = match files::init_runtime() {
        Ok(lib) => lib,
        Err(e) => {
            eprintln!("SKIPPED — {e}");
            return;
        }
    };
    eprintln!("voice: {}\nruntime: {}", model.display(), runtime.display());

    let started = std::time::Instant::now();
    let mut voice = PiperVoice::open(&model, &config).unwrap_or_else(|e| panic!("{e}"));
    let loaded = started.elapsed();
    let audio = voice
        .speak("The LORD is my shepherd; I shall lack nothing. He makes me lie down in green pastures.")
        .unwrap_or_else(|e| panic!("{e}"));
    let spoken = started.elapsed() - loaded;

    eprintln!(
        "{}: {} samples at {} Hz = {:.2} s, peak {:.2}; model loaded in {:.2} s, spoken in {:.2} s",
        voice.name(),
        audio.samples.len(),
        audio.sample_rate,
        audio.duration_secs(),
        audio.peak(),
        loaded.as_secs_f32(),
        spoken.as_secs_f32()
    );
    assert_eq!(audio.sample_rate, 22_050, "LJ Speech high is a 22.05 kHz voice");
    assert!(audio.duration_secs() > 2.0 && audio.duration_secs() < 12.0, "duration looks wrong");
    assert!(audio.peak() > 0.05, "audio is silent");

    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/voice-check.wav");
    std::fs::write(&out, audio.to_wav()).expect("write wav");
    eprintln!("wrote {}", out.display());
}
