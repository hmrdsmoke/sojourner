// ── Add your header block above ──

//! Speech: turning the words of a verse into sound.
//!
//! The reader speaks a verse at a time, so the page can show which verse is
//! being read and the listener can pause, go back a verse, or skip ahead.
//! What makes the sound is behind the `Voice` trait, and one engine
//! implements it today: `piper`, a local neural voice that runs entirely on
//! this machine. Nothing here touches the network. `reader` is the thread
//! that reads a chapter aloud with it, verse by verse, and tells the window
//! which verse it is on.
//!
//! Where the voice files and the runtime library are looked for is in
//! `files`; what they are, where they came from, and under what terms is in
//! assets/SOURCES.md.

pub mod files;
pub mod names;
pub mod phonemes;
pub mod piper;
pub mod reader;

/// Mono PCM audio, as the engines produce it.
#[derive(Debug, Clone)]
pub struct Audio {
    pub sample_rate: u32,
    /// Samples in -1.0..=1.0.
    pub samples: Vec<f32>,
}

impl Audio {
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate as f32
    }

    /// The loudest sample, as a quick check that something was said.
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()))
    }

    /// The audio as a 16-bit PCM WAV file, for listening to outside the app.
    pub fn to_wav(&self) -> Vec<u8> {
        let data_len = (self.samples.len() * 2) as u32;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&(self.sample_rate * 2).to_le_bytes()); // bytes per second
        out.extend_from_slice(&2u16.to_le_bytes()); // block align
        out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in &self.samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}

/// An engine that can say a piece of text.
pub trait Voice: Send {
    /// A name for the About page: which voice, which engine.
    fn name(&self) -> &str;
    /// Say the text. One verse at a time is the intended grain.
    fn speak(&mut self, text: &str) -> Result<Audio, Error>;
}

#[derive(Debug)]
pub enum Error {
    /// A file that should be there isn't. The message says which and where
    /// it was looked for.
    Missing(String),
    /// The engine failed.
    Engine(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(what) => write!(f, "missing: {what}"),
            Error::Engine(what) => write!(f, "speech engine: {what}"),
        }
    }
}

impl std::error::Error for Error {}
