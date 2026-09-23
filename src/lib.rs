// ── Add your header block above ──

//! Sojourner's library half. The app binary (`main.rs`) is the COSMIC
//! window; everything that doesn't need a window — the Bible text, its
//! parser, and later the cross-reference web and TTS — lives here so it can
//! be tested without one.

pub mod crossrefs;
pub mod text;
pub mod voice;

/// Everything the reader needs, loaded together: the text, the index over
/// it, and the cross-reference web resolved against it.
#[derive(Debug)]
pub struct Library {
    pub bible: text::Bible,
    pub index: text::VerseIndex,
    pub refs: crossrefs::CrossRefs,
}

impl Library {
    /// Parse the bundled text and cross-references. About half a second in
    /// a release build.
    pub fn load_bundled() -> Result<Library, String> {
        let bible = text::usfm::load_bundled().map_err(|e| e.to_string())?;
        let index = text::VerseIndex::new(&bible);
        let refs = crossrefs::CrossRefs::load_bundled(&index).map_err(|e| e.to_string())?;
        Ok(Library { bible, index, refs })
    }
}
