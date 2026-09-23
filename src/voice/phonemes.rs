// ── Add your header block above ──

//! Words to phonemes, the way Piper's own program does it — with the
//! punctuation kept.
//!
//! The voice model was trained on phoneme strings that carry the clause
//! punctuation of the text (`,` `.` `;` `:` `?` `!`) and a space between
//! clauses; that is how it learned where to pause and how a sentence falls
//! at its end. espeak-ng's `espeak_TextToPhonemes` hands back one clause at
//! a time with the punctuation stripped, and the piper-rs crate simply
//! glues those clauses together — so a verse reached the model as one
//! breathless run with the words at each clause boundary fused
//! ("beginningGod"). That is why the first version sounded like it had no
//! grammar. This module does what Piper does: it asks espeak-ng for the
//! clauses, works out which punctuation ended each one, and hands the model
//! one sentence at a time with that punctuation in place.
//!
//! espeak-ng is a single global; every call goes through one lock and the
//! library is initialized once.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use super::names::Names;
use super::Error;

/// The result of initializing espeak-ng, kept so it happens once.
static INIT: OnceLock<Result<(), String>> = OnceLock::new();

// espeak-ng reads `[[phonemes]]` in its input only when this global of its
// is set, and only `espeak_Synth` sets it (from the `espeakPHONEMES` flag);
// `espeak_TextToPhonemes`, the call this module makes, never touches it.
// Sojourner never synthesizes through espeak-ng, so it sets the flag once
// after initializing and it stays set. The global is espeak-ng 1.52's
// (`translate.c`, statically linked in by espeak-rs-sys 0.2.0);
// tests/names.rs proves it still works whenever that crate is updated.
unsafe extern "C" {
    static mut option_phoneme_input: std::ffi::c_int;
}

/// espeak-ng keeps its state in globals; one caller at a time.
static ESPEAK: Mutex<()> = Mutex::new(());

/// Initialize espeak-ng. `data_dir` is the `espeak-ng-data` directory to
/// use; `None` lets espeak-ng find its own (the `ESPEAK_DATA_PATH`
/// variable, else the path compiled in at build time). Harmless to call
/// again.
pub fn init(data_dir: Option<&Path>) -> Result<(), Error> {
    INIT.get_or_init(|| {
        let _guard = ESPEAK.lock().unwrap_or_else(|e| e.into_inner());
        let path = data_dir.and_then(|d| CString::new(d.to_string_lossy().as_ref()).ok());
        let path_ptr = path.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        // SAFETY: plain C call with a valid (or null) path; espeak-ng copies
        // the string.
        let sample_rate = unsafe {
            espeak_rs_sys::espeak_Initialize(
                espeak_rs_sys::espeak_AUDIO_OUTPUT_AUDIO_OUTPUT_RETRIEVAL,
                0,
                path_ptr,
                espeak_rs_sys::espeakINITIALIZE_DONT_EXIT as i32,
            )
        };
        if sample_rate > 0 {
            // SAFETY: a plain int in espeak-ng's globals, written under the
            // same lock every other call to espeak-ng takes.
            unsafe {
                option_phoneme_input = 1;
            }
            Ok(())
        } else {
            Err(format!(
                "espeak-ng could not start (code {sample_rate}); its espeak-ng-data directory was {}",
                match data_dir {
                    Some(d) => d.display().to_string(),
                    None => "the one compiled in".to_string(),
                }
            ))
        }
    })
    .clone()
    .map_err(Error::Engine)
}

/// The text as phoneme strings, one per sentence, with Piper's punctuation
/// convention: each clause's phonemes followed by the punctuation that
/// ended it, a space after a clause-internal mark (`,` `:` `;`), and a
/// sentence ending at `.` `?` `!` or the end of the text. `voice` is the
/// espeak-ng voice named in the model's configuration ("en"). Names in the
/// bundled table (`names.rs`) are said as the table says, and words set in
/// capitals are read as words.
pub fn sentences(text: &str, voice: &str) -> Result<Vec<String>, Error> {
    let respelled = Names::bundled().respell(&as_words(text));
    sentences_of(&respelled, voice)
}

/// A word set in capitals — the text's "LORD" and "GOD" for the divine
/// name, "HOLY TO THE LORD" on the high priest's plate — read as the word
/// it is, not as letters. espeak-ng takes a short all-capital word after
/// another capitalized word for an acronym and spells it ("Lord G-O-D"),
/// so every run of two or more capitals is set to an initial capital
/// before espeak-ng sees it. Speech only; the page keeps the capitals.
pub fn as_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if word.chars().count() >= 2 && word.chars().all(|c| c.is_uppercase()) {
            let mut chars = word.chars();
            out.extend(chars.next());
            out.extend(chars.flat_map(char::to_lowercase));
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_alphabetic() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// `sentences`, with the text taken as it is — no name table.
pub fn sentences_of(text: &str, voice: &str) -> Result<Vec<String>, Error> {
    let _guard = ESPEAK.lock().unwrap_or_else(|e| e.into_inner());

    let voice_c = CString::new(voice).map_err(|_| Error::Engine("voice name has a NUL in it".into()))?;
    // SAFETY: plain C call with a valid string.
    let set = unsafe { espeak_rs_sys::espeak_SetVoiceByName(voice_c.as_ptr()) };
    if set != espeak_rs_sys::espeak_ERROR_EE_OK {
        return Err(Error::Engine(format!("espeak-ng has no voice \"{voice}\"")));
    }

    let text_c = CString::new(text.replace('\0', " ")).map_err(|_| Error::Engine("text has a NUL in it".into()))?;
    let base = text_c.as_ptr();
    let mut cursor: *const c_char = base;
    let bytes = text.as_bytes();

    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut consumed_from = 0usize;

    while !cursor.is_null() {
        // SAFETY: `cursor` points into `text_c`, which outlives the loop;
        // espeak-ng advances it past the clause it translated and returns a
        // pointer into its own buffer, valid until the next call.
        let result = unsafe {
            espeak_rs_sys::espeak_TextToPhonemes(
                &mut cursor as *mut *const c_char as *mut *const c_void,
                espeak_rs_sys::espeakCHARS_UTF8 as i32,
                espeak_rs_sys::espeakPHONEMES_IPA as i32,
            )
        };
        let phonemes = if result.is_null() {
            String::new()
        } else {
            // SAFETY: a NUL-terminated string espeak-ng owns, read before the next call.
            unsafe { CStr::from_ptr(result) }.to_string_lossy().into_owned()
        };
        let phonemes = strip_language_switches(&phonemes);

        // The text this clause came from: everything espeak-ng advanced
        // over. When there is more text to come it has read one character
        // past the clause as lookahead (a letter of the next word, an
        // opening quote, a space); that character is not part of this
        // clause. Only the end of the text matters here, for the
        // punctuation; the phonemes themselves are espeak-ng's.
        let consumed_to = if cursor.is_null() {
            bytes.len()
        } else {
            ((cursor as usize) - (base as usize)).min(bytes.len())
        };
        let mut clause_text = text.get(consumed_from..consumed_to).unwrap_or("");
        if !cursor.is_null() {
            if let Some(last) = clause_text.chars().last().filter(|c| !is_terminator(*c)) {
                clause_text = &clause_text[..clause_text.len() - last.len_utf8()];
            }
        }
        consumed_from = consumed_to;

        let terminator = terminator_of(clause_text);
        let ends_sentence = matches!(terminator, Some('.' | '?' | '!'));

        if !phonemes.trim().is_empty() {
            if !current.is_empty() && !current.ends_with(' ') {
                current.push(' ');
            }
            current.push_str(phonemes.trim());
            if let Some(mark) = terminator {
                current.push(mark);
            }
        }
        if ends_sentence && !current.is_empty() {
            sentences.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    Ok(sentences)
}

/// The clause punctuation the voice knows.
fn is_terminator(c: char) -> bool {
    matches!(c, '.' | '?' | '!' | ',' | ':' | ';')
}

/// The punctuation that ended a clause, read from the end of the text it
/// came from, looking past quotes, brackets and whitespace.
fn terminator_of(clause_text: &str) -> Option<char> {
    clause_text
        .chars()
        .rev()
        .find(|c| !c.is_whitespace() && !matches!(c, '”' | '“' | '"' | '’' | '‘' | '\'' | ')' | '(' | ']' | '[' | '»' | '«'))
        .filter(|c| is_terminator(*c))
}

/// espeak-ng marks a run of words it read in another language with
/// "(en)"-style tags. They are not phonemes.
fn strip_language_switches(phonemes: &str) -> String {
    let mut out = String::with_capacity(phonemes.len());
    let mut depth = 0usize;
    for c in phonemes.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}
