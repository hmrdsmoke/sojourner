// ── Add your header block above ──

//! The piper engine: a VITS voice model run by ONNX Runtime, with espeak-ng
//! turning words into phonemes first. Everything runs on this machine.
//!
//! Lineage: the Piper project (Open Home Foundation, GPL-3.0), driven here
//! through the piper-rs crate (MIT), which compiles in espeak-ng (GPL-3.0)
//! and loads ONNX Runtime (MIT). The voice model has its own card. See
//! assets/SOURCES.md for all of it.
//!
//! piper-rs is used for what it does well — reading the model's
//! configuration and running the model on a phoneme string. The phonemes
//! themselves come from `phonemes.rs`, which keeps the punctuation the way
//! Piper's own program does; piper-rs's text path drops it, and with it the
//! pauses and the shape of every sentence.

use std::path::Path;
use std::time::Duration;

use super::{files, phonemes, Audio, Error, Voice};

/// Silence between one sentence and the next within a verse. Piper's own
/// default for the same gap.
const SENTENCE_SILENCE: Duration = Duration::from_millis(200);

/// How long each sound is held, relative to the model's own pace: 1.0 is
/// the pace the voice was trained at; larger is slower.
const PACE: f32 = 1.0;

pub struct PiperVoice {
    name: String,
    /// The espeak-ng voice the model was trained with ("en"), from its
    /// configuration. The phonemes must come from the same one.
    espeak_voice: String,
    piper: piper_rs::Piper,
}

impl PiperVoice {
    /// Open a voice from its model and config. `files::init_runtime` must
    /// have been called first.
    pub fn open(model: &Path, config: &Path) -> Result<Self, Error> {
        let file = std::fs::File::open(config)
            .map_err(|e| Error::Missing(format!("voice config {}: {e}", config.display())))?;
        let settings: piper_rs::ModelConfig =
            serde_json::from_reader(file).map_err(|e| Error::Engine(format!("voice config: {e}")))?;
        let espeak_voice = settings.espeak.voice.clone();

        phonemes::init(files::find_espeak_data().as_deref())?;

        let piper = piper_rs::Piper::new(model, config).map_err(|e| Error::Engine(e.to_string()))?;
        let name = model
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "piper".to_string());
        Ok(PiperVoice { name, espeak_voice, piper })
    }
}

impl Voice for PiperVoice {
    fn name(&self) -> &str {
        &self.name
    }

    /// One inference per sentence, as Piper does it, with a short silence
    /// between sentences. A verse with no words to say comes back empty.
    fn speak(&mut self, text: &str) -> Result<Audio, Error> {
        let sentences = phonemes::sentences(text, &self.espeak_voice)?;
        let mut out = Audio { sample_rate: 0, samples: Vec::new() };
        for (i, sentence) in sentences.iter().enumerate() {
            let (samples, sample_rate) = self
                .piper
                .create(sentence, true, None, Some(PACE), None, None)
                .map_err(|e| Error::Engine(e.to_string()))?;
            if i > 0 {
                let gap = (sample_rate as f32 * SENTENCE_SILENCE.as_secs_f32()) as usize;
                out.samples.extend(std::iter::repeat_n(0.0f32, gap));
            }
            out.sample_rate = sample_rate;
            out.samples.extend(samples);
        }
        Ok(out)
    }
}
