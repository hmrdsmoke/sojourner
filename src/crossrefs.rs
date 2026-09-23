// ── Add your header block above ──

//! The cross-reference web: which verses point at which.
//!
//! Source: openbible.info's cross-reference set (CC BY 4.0), compiled
//! primarily from the Treasury of Scripture Knowledge, with a relevance
//! vote per link. See assets/SOURCES.md for the file, its hash and its
//! lineage. The zip is compiled into the binary and parsed at startup.
//!
//! The file is one reference per line, tab-separated:
//!
//! ```text
//! From Verse   To Verse              Votes   #www.openbible.info CC-BY 2026-09-21
//! Gen.1.1      Rom.1.19-Rom.1.20     59
//! ```
//!
//! Books use OSIS abbreviations (`Gen`, `1Kgs`, `Phlm`); the target may be a
//! range, and a range may run across a book boundary (`Acts.28.17-Rom.1.1`).
//! Every reference is resolved against the parsed Bible through a
//! `VerseIndex`.
//!
//! The set is numbered the KJV/ESV way, and the WEB differs in two places —
//! both explained by the translators' own footnotes at those verses. Those
//! references are *renumbered*, by the table in `to_web_numbering`, and
//! counted in `CrossRefs::renumbered`. Anything that still fails to resolve
//! is never dropped silently: it is kept in `CrossRefs::unresolved`, and the
//! test pins the exact list (which is expected to be empty).
//!
//! Links for a verse are stored most-voted first, so the trail can show the
//! references readers found most relevant before the rest.

use std::collections::HashMap;
use std::io::Read;

use crate::text::{VerseIndex, VerseRef};

/// The openbible.info zip, compiled into the binary (2 MB).
pub const BUNDLED_ZIP: &[u8] = include_bytes!("../assets/cross-references.zip");

/// One link out of a verse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    /// First (or only) verse of the target.
    pub target: VerseRef,
    /// Last verse of the target. Equal to `target` unless the file gave a
    /// range like `Rom.1.19-Rom.1.20`.
    pub target_end: VerseRef,
    /// openbible.info's relevance vote. Higher is more relevant; can be
    /// negative.
    pub votes: i32,
}

/// A line that could not be resolved against the text, kept for the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub line: usize,
    pub from: String,
    pub to: String,
    pub reason: String,
}

#[derive(Debug)]
pub struct CrossRefs {
    by_source: HashMap<VerseRef, Vec<Link>>,
    /// Links that resolved, in total.
    pub resolved: usize,
    /// Verse numbers translated by `to_web_numbering` on the way in.
    pub renumbered: usize,
    /// Lines that did not resolve, in file order.
    pub unresolved: Vec<Unresolved>,
    /// The date stamped in the file's header, e.g. "2026-09-21".
    pub snapshot: String,
}

#[derive(Debug)]
pub enum Error {
    Zip(String),
    Parse { line: usize, what: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Zip(e) => write!(f, "cross-references zip: {e}"),
            Error::Parse { line, what } => write!(f, "cross_references.txt:{line}: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl CrossRefs {
    /// Load the bundled set, resolving every reference against `index`.
    pub fn load_bundled(index: &VerseIndex) -> Result<Self, Error> {
        let zip_err = |e: zip::result::ZipError| Error::Zip(e.to_string());
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(BUNDLED_ZIP)).map_err(zip_err)?;
        let mut text = String::new();
        archive
            .by_name("cross_references.txt")
            .map_err(zip_err)?
            .read_to_string(&mut text)
            .map_err(|e| Error::Zip(e.to_string()))?;
        Self::parse(&text, index)
    }

    /// Parse the tab-separated text.
    pub fn parse(text: &str, index: &VerseIndex) -> Result<Self, Error> {
        let mut lines = text.lines().enumerate();

        // Header: "From Verse\tTo Verse\tVotes\t#www.openbible.info CC-BY 2026-09-21"
        let (_, header) = lines
            .next()
            .ok_or_else(|| Error::Parse { line: 1, what: "empty file".into() })?;
        if !header.starts_with("From Verse\tTo Verse\tVotes") {
            return Err(Error::Parse { line: 1, what: format!("unexpected header {header:?}") });
        }
        let snapshot = header
            .rsplit(' ')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let mut by_source: HashMap<VerseRef, Vec<Link>> = HashMap::new();
        let mut resolved = 0usize;
        let mut renumbered = 0usize;
        let mut unresolved = Vec::new();

        for (i, line) in lines {
            let line_no = i + 1;
            if line.trim().is_empty() {
                continue;
            }
            let mut cols = line.split('\t');
            let (Some(from), Some(to), Some(votes)) = (cols.next(), cols.next(), cols.next()) else {
                return Err(Error::Parse { line: line_no, what: format!("expected 3 columns: {line:?}") });
            };
            let votes: i32 = votes
                .trim()
                .parse()
                .map_err(|_| Error::Parse { line: line_no, what: format!("bad vote count {votes:?}") })?;

            let record = |reason: String| Unresolved {
                line: line_no,
                from: from.to_string(),
                to: to.to_string(),
                reason,
            };

            let source = match resolve_one(from, index, &mut renumbered) {
                Ok(v) => v,
                Err(reason) => {
                    unresolved.push(record(format!("from: {reason}")));
                    continue;
                }
            };
            let (target, target_end) = match resolve_target(to, index, &mut renumbered) {
                Ok(pair) => pair,
                Err(reason) => {
                    unresolved.push(record(format!("to: {reason}")));
                    continue;
                }
            };

            by_source.entry(source).or_default().push(Link { target, target_end, votes });
            resolved += 1;
        }

        // Most-voted first; stable, so equal votes keep file order.
        for links in by_source.values_mut() {
            links.sort_by(|a, b| b.votes.cmp(&a.votes));
        }

        Ok(CrossRefs { by_source, resolved, renumbered, unresolved, snapshot })
    }

    /// The links out of one verse, most-voted first. Empty if none.
    pub fn from(&self, verse: VerseRef) -> &[Link] {
        self.by_source.get(&verse).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Number of verses that have at least one link out.
    pub fn source_count(&self) -> usize {
        self.by_source.len()
    }
}

/// `Rom.1.19-Rom.1.20` → (Rom 1:19, Rom 1:20); `Ps.90.2` → (Ps 90:2, Ps 90:2).
/// A range may cross into the next book (`2Chr.36.22-Ezra.1.3`) as long as
/// it runs forward through the canon.
fn resolve_target(to: &str, index: &VerseIndex, renumbered: &mut usize) -> Result<(VerseRef, VerseRef), String> {
    match to.split_once('-') {
        Some((a, b)) => {
            let start = resolve_one(a, index, renumbered)?;
            let end = resolve_one(b, index, renumbered)?;
            if end < start {
                return Err(format!("range runs backwards: {to}"));
            }
            Ok((start, end))
        }
        None => {
            let one = resolve_one(to, index, renumbered)?;
            Ok((one, one))
        }
    }
}

/// Where the set's KJV/ESV-style numbering differs from the WEB's. Each line
/// is explained by the translators' footnote at the verse in question:
///
/// * Romans 16:25–27, the doxology. The WEB follows the Majority Text and
///   places it after 14:23, as 14:24–26; the footnote at 16:25 says so. The
///   set has it at the end of chapter 16.
/// * 3 John 15. The set splits 3 John's last verse in two (14, 15); the WEB
///   keeps the traditional single verse 14.
fn to_web_numbering(code: &str, chapter: u32, verse: u32) -> (u32, u32) {
    match (code, chapter, verse) {
        ("ROM", 16, 25) => (14, 24),
        ("ROM", 16, 26) => (14, 25),
        ("ROM", 16, 27) => (14, 26),
        ("3JN", 1, 15) => (1, 14),
        _ => (chapter, verse),
    }
}

/// `Gen.1.1` → the verse, if the Bible has it.
fn resolve_one(reference: &str, index: &VerseIndex, renumbered: &mut usize) -> Result<VerseRef, String> {
    let mut parts = reference.split('.');
    let (Some(osis), Some(chapter), Some(verse), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("malformed reference {reference:?}"));
    };
    let code = usfm_code(osis).ok_or_else(|| format!("unknown book {osis:?}"))?;
    let book = index.book(code).ok_or_else(|| format!("book {code} not in the Bible"))?;
    let chapter: u32 = chapter.parse().map_err(|_| format!("bad chapter in {reference:?}"))?;
    let verse: u32 = verse.parse().map_err(|_| format!("bad verse in {reference:?}"))?;
    let (web_chapter, web_verse) = to_web_numbering(code, chapter, verse);
    if (web_chapter, web_verse) != (chapter, verse) {
        *renumbered += 1;
    }
    let r = VerseRef { book, chapter: web_chapter, verse: web_verse };
    if index.contains(r) {
        Ok(r)
    } else {
        Err(format!("{reference} is not a verse in this Bible"))
    }
}

/// OSIS book abbreviation → USFM code, for the 66 books the set covers.
fn usfm_code(osis: &str) -> Option<&'static str> {
    const MAP: [(&str, &str); 66] = [
        ("Gen", "GEN"), ("Exod", "EXO"), ("Lev", "LEV"), ("Num", "NUM"), ("Deut", "DEU"),
        ("Josh", "JOS"), ("Judg", "JDG"), ("Ruth", "RUT"), ("1Sam", "1SA"), ("2Sam", "2SA"),
        ("1Kgs", "1KI"), ("2Kgs", "2KI"), ("1Chr", "1CH"), ("2Chr", "2CH"), ("Ezra", "EZR"),
        ("Neh", "NEH"), ("Esth", "EST"), ("Job", "JOB"), ("Ps", "PSA"), ("Prov", "PRO"),
        ("Eccl", "ECC"), ("Song", "SNG"), ("Isa", "ISA"), ("Jer", "JER"), ("Lam", "LAM"),
        ("Ezek", "EZK"), ("Dan", "DAN"), ("Hos", "HOS"), ("Joel", "JOL"), ("Amos", "AMO"),
        ("Obad", "OBA"), ("Jonah", "JON"), ("Mic", "MIC"), ("Nah", "NAM"), ("Hab", "HAB"),
        ("Zeph", "ZEP"), ("Hag", "HAG"), ("Zech", "ZEC"), ("Mal", "MAL"),
        ("Matt", "MAT"), ("Mark", "MRK"), ("Luke", "LUK"), ("John", "JHN"), ("Acts", "ACT"),
        ("Rom", "ROM"), ("1Cor", "1CO"), ("2Cor", "2CO"), ("Gal", "GAL"), ("Eph", "EPH"),
        ("Phil", "PHP"), ("Col", "COL"), ("1Thess", "1TH"), ("2Thess", "2TH"), ("1Tim", "1TI"),
        ("2Tim", "2TI"), ("Titus", "TIT"), ("Phlm", "PHM"), ("Heb", "HEB"), ("Jas", "JAS"),
        ("1Pet", "1PE"), ("2Pet", "2PE"), ("1John", "1JN"), ("2John", "2JN"), ("3John", "3JN"),
        ("Jude", "JUD"), ("Rev", "REV"),
    ];
    MAP.iter().find(|(o, _)| *o == osis).map(|(_, code)| *code)
}
