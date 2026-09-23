// ── Add your header block above ──

//! The names of scripture, said right.
//!
//! espeak-ng knows no names. It reads "Job" by the rules for English
//! spelling and gets the thing you go to in the morning. So Sojourner
//! carries a table: a word as it is spelled in the text, and the sounds to
//! say it with, in espeak-ng's own phoneme mnemonics. Before a verse goes to
//! espeak-ng, every word in the table is replaced by its sounds in double
//! square brackets — espeak-ng's notation for "these are phonemes, not
//! spelling" — and the rest of the clause is read as usual, punctuation and
//! all. The words on the page never change; only what the voice is told.
//!
//! Every row names its source. Where the sounds come from is recorded in
//! assets/SOURCES.md, section 5, like everything else Sojourner did not
//! author.
//!
//! The table is `assets/names.tsv`: tab-separated, `#` comments, four
//! columns — the word, the mnemonics, the pronunciation as the source wrote
//! it, and the source's id.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The table shipped with the app, compiled in.
const BUNDLED: &str = include_str!("../../assets/names.tsv");

/// A word → the espeak-ng mnemonics to say it with.
#[derive(Debug, Default)]
pub struct Names {
    map: HashMap<String, String>,
}

impl Names {
    /// The bundled table, parsed once. A malformed table is a build defect
    /// and stops the app the first time a name is looked up.
    pub fn bundled() -> &'static Names {
        static NAMES: OnceLock<Names> = OnceLock::new();
        NAMES.get_or_init(|| Names::parse(BUNDLED).unwrap_or_else(|e| panic!("assets/names.tsv: {e}")))
    }

    /// Parse a table. Blank lines and `#` comments are skipped; every other
    /// line needs four tab-separated columns, and the mnemonics must be
    /// plain ASCII (espeak-ng reads nothing else inside `[[ ]]`).
    pub fn parse(tsv: &str) -> Result<Names, String> {
        let mut map = HashMap::new();
        for (n, line) in tsv.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() != 4 {
                return Err(format!("line {}: expected 4 columns, found {}", n + 1, cols.len()));
            }
            let (word, phonemes) = (cols[0].trim(), cols[1].trim());
            if word.is_empty() || phonemes.is_empty() {
                return Err(format!("line {}: empty word or phonemes", n + 1));
            }
            if !phonemes.is_ascii() || phonemes.contains(['[', ']', ' ']) {
                return Err(format!("line {}: mnemonics must be ASCII without brackets or spaces: {phonemes:?}", n + 1));
            }
            if map.insert(word.to_string(), phonemes.to_string()).is_some() {
                return Err(format!("line {}: {word} appears twice", n + 1));
            }
        }
        Ok(Names { map })
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The mnemonics for a word exactly as spelled (case matters: "Job" is
    /// a man, "job" is work).
    pub fn get(&self, word: &str) -> Option<&str> {
        self.map.get(word).map(String::as_str)
    }

    /// The text with every word in the table replaced by `[[mnemonics]]`.
    /// A possessive ("Job’s") is folded into the brackets, since espeak-ng
    /// would read a bare "’s" after them as the letter S.
    pub fn respell(&self, text: &str) -> String {
        if self.map.is_empty() {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len() + 32);
        let mut rest = text;
        while !rest.is_empty() {
            // Copy everything up to the next letter.
            let start = rest.find(|c: char| c.is_alphabetic()).unwrap_or(rest.len());
            out.push_str(&rest[..start]);
            rest = &rest[start..];
            if rest.is_empty() {
                break;
            }
            // The word: a run of letters.
            let end = rest.find(|c: char| !c.is_alphabetic()).unwrap_or(rest.len());
            let (word, after) = rest.split_at(end);
            // A possessive right after it?
            let possessive = ["’s", "'s"].into_iter().find(|p| {
                after.strip_prefix(*p).is_some_and(|tail| !tail.starts_with(|c: char| c.is_alphabetic()))
            });
            match self.map.get(word) {
                Some(phonemes) => {
                    out.push_str("[[");
                    out.push_str(phonemes);
                    if possessive.is_some() {
                        out.push_str(possessive_ending(phonemes));
                    }
                    out.push_str("]]");
                    rest = &after[possessive.map_or(0, str::len)..];
                }
                None => {
                    out.push_str(word);
                    rest = after;
                }
            }
        }
        out
    }
}

/// The sound of a possessive "’s" after these mnemonics: "iz" after a
/// hissing sound (Moses’s), "s" after a voiceless one (Lot’s), "z"
/// otherwise (Job’s).
fn possessive_ending(phonemes: &str) -> &'static str {
    let last = phonemes.trim_end_matches([':', '#']);
    if last.ends_with("tS") || last.ends_with("dZ") || last.ends_with(['s', 'z', 'S', 'Z']) {
        "Iz"
    } else if last.ends_with(['p', 't', 'k', 'f', 'T']) {
        "s"
    } else {
        "z"
    }
}
