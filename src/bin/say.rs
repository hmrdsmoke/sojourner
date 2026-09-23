// ── Add your header block above ──

//! Says something with the bundled voice and writes it to a WAV file, for
//! listening to a change without opening the app:
//!
//!     cargo run --bin say -- "Then Job answered the LORD." [out.wav]
//!
//! The text goes through exactly what the reader uses — the names table,
//! the phonemizer, the voice — so what you hear is what a verse gets. The
//! file is `target/say.wav` unless a path is given. Needs the voice files
//! and ONNX Runtime in place (src/voice/files.rs says where).

use sojourner::voice::{files, piper::PiperVoice, Voice};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(text) = args.next() else {
        eprintln!("usage: say \"text\" [out.wav]");
        std::process::exit(2);
    };
    let out = args.next().unwrap_or_else(|| "target/say.wav".to_string());

    let run = || -> Result<(), Box<dyn std::error::Error>> {
        files::init_runtime()?;
        let (model, config) = files::find_voice()?;
        let mut voice = PiperVoice::open(&model, &config)?;
        let audio = voice.speak(&text)?;
        std::fs::write(&out, audio.to_wav())?;
        eprintln!("{}: {:.1} s → {out}", voice.name(), audio.duration_secs());
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
