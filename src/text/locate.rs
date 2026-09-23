// ── Add your header block above ──

//! Finding things in the text: one verse's content by reference, a
//! human-readable label for a reference or a range, and the translators' own
//! cross-reference targets ("Isaiah 7:14; 9:6") turned into references the
//! app can follow.

use super::index::VerseIndex;
use super::model::*;

// ────────────────────────────────────────────────────────────────────────────
// One verse's content
// ────────────────────────────────────────────────────────────────────────────

/// One verse's content, block by block. `pieces` are slices into the
/// chapter's blocks, so a verse that runs across poetry lines keeps its line
/// structure. The first piece begins with the verse's own marker.
#[derive(Debug)]
pub struct VerseText<'a> {
    /// The marker the verse was found under. For a bridged verse (`\v 15-16`)
    /// asked for as 16, this is 15–16.
    pub number: VerseNumber,
    pub pieces: Vec<(BlockKind, &'a [Inline])>,
}

impl VerseText<'_> {
    /// Plain words only — no footnotes, no markers — with a space where the
    /// verse crosses a block boundary. The same rule as `plain_verses`.
    pub fn plain(&self) -> String {
        let mut out = String::new();
        for (_, inlines) in &self.pieces {
            let mut need_space = true;
            for inline in *inlines {
                if let Inline::Text(run) = inline {
                    if need_space && !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push_str(&run.text);
                    need_space = false;
                }
            }
        }
        out
    }
}

/// The content of one verse: from its marker up to the next verse's marker,
/// through as many verse-flow blocks as it runs. `None` if the book, chapter
/// or verse doesn't exist.
pub fn verse(bible: &Bible, at: VerseRef) -> Option<VerseText<'_>> {
    let book = bible.books.get(usize::from(at.book))?;
    let chapter = book.chapters.iter().find(|c| c.number == at.chapter)?;

    let mut number: Option<VerseNumber> = None;
    let mut pieces = Vec::new();

    for block in &chapter.blocks {
        if !block.kind.is_verse_flow() {
            continue;
        }
        // If we're already inside the verse, this block continues it from
        // its first inline.
        let mut start: Option<usize> = number.map(|_| 0);

        for (i, inline) in block.content.iter().enumerate() {
            let Inline::Verse(n) = inline else { continue };
            if number.is_some() {
                // The next verse begins here: close ours and finish.
                if let Some(s) = start.filter(|&s| s < i) {
                    pieces.push((block.kind, &block.content[s..i]));
                }
                return number.map(|number| VerseText { number, pieces });
            }
            if n.start <= at.verse && at.verse <= n.end {
                number = Some(*n);
                start = Some(i);
            }
        }

        if let Some(s) = start.filter(|&s| s < block.content.len()) {
            pieces.push((block.kind, &block.content[s..]));
        }
    }

    number.map(|number| VerseText { number, pieces })
}

// ────────────────────────────────────────────────────────────────────────────
// Labels
// ────────────────────────────────────────────────────────────────────────────

/// The name a reference is written with: the publisher's chapter label where
/// there is one ("Psalm"), else the book's short name ("1 Kings").
fn book_name(bible: &Bible, book: u8) -> &str {
    let book = &bible.books[usize::from(book)];
    book.chapter_label.as_deref().unwrap_or(&book.names.short)
}

/// "John 3:16", "Romans 1:19–20", "Luke 24:44–25:2",
/// "2 Chronicles 36:22 – Ezra 1:3".
pub fn reference_label(bible: &Bible, start: VerseRef, end: VerseRef) -> String {
    let name = book_name(bible, start.book);
    if start == end {
        format!("{name} {}:{}", start.chapter, start.verse)
    } else if start.book == end.book && start.chapter == end.chapter {
        format!("{name} {}:{}\u{2013}{}", start.chapter, start.verse, end.verse)
    } else if start.book == end.book {
        format!("{name} {}:{}\u{2013}{}:{}", start.chapter, start.verse, end.chapter, end.verse)
    } else {
        format!(
            "{name} {}:{} \u{2013} {} {}:{}",
            start.chapter,
            start.verse,
            book_name(bible, end.book),
            end.chapter,
            end.verse
        )
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The translators' own cross-reference targets
// ────────────────────────────────────────────────────────────────────────────

/// One target out of a `\xt` note. `range` is `None` when the text could not
/// be turned into verses that exist; the label is still shown as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub label: String,
    pub range: Option<(VerseRef, VerseRef)>,
}

/// Turn a `\xt` string into references. The publisher writes them as a
/// semicolon list — "Exodus 20:8-11; Deuteronomy 5:12-15" — where an item
/// without a book name continues the previous book ("Isaiah 7:14; 9:6"), a
/// chapter's verses may be a comma list with ranges ("Ezekiel 1:5-10,22"),
/// a range may cross chapters with an em-dash ("1 Kings 8:1—9:9"), and an
/// item may carry a "See " prefix or an " LXX" suffix. Book names are the
/// short names the files themselves declare (`\toc2`).
pub fn parse_targets(bible: &Bible, index: &VerseIndex, text: &str) -> Vec<Target> {
    // Longest names first, so "1 Kings" can't be read as some shorter name
    // followed by digits.
    let mut names: Vec<(&str, u8)> = bible
        .books
        .iter()
        .enumerate()
        .filter(|(_, book)| !book.chapters.is_empty())
        .map(|(i, book)| (book.names.short.as_str(), i as u8))
        .collect();
    names.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));

    let mut out = Vec::new();
    let mut book: Option<u8> = None;

    for item in text.split(';') {
        let written = item.trim();
        let mut rest = written.strip_prefix("See ").unwrap_or(written).trim();
        let lxx = rest.ends_with(" LXX");
        if lxx {
            rest = rest.trim_end_matches(" LXX").trim();
        }

        if let Some((name, b)) = names
            .iter()
            .find(|(name, _)| rest.strip_prefix(*name).is_some_and(|after| after.starts_with(' ')))
        {
            book = Some(*b);
            rest = rest[name.len()..].trim();
        }

        let parsed = book.and_then(|b| parse_chapter_verses(index, b, rest, lxx, bible));
        match parsed {
            Some(targets) => out.extend(targets),
            None => out.push(Target {
                label: written.to_string(),
                range: None,
            }),
        }
    }
    out
}

/// "20:8-11,13" for one book → the targets it names.
fn parse_chapter_verses(index: &VerseIndex, book: u8, spec: &str, lxx: bool, bible: &Bible) -> Option<Vec<Target>> {
    let (chapter, verses) = spec.split_once(':')?;
    let chapter: u32 = chapter.trim().parse().ok()?;
    let mut out = Vec::new();

    for item in verses.split(',') {
        let item = item.trim();
        // "8-11", "8—11", or a cross-chapter "1—9:9".
        let (first, last) = match item.split_once(['-', '\u{2014}']) {
            Some((a, b)) => (a.trim(), Some(b.trim())),
            None => (item, None),
        };
        let start = VerseRef {
            book,
            chapter,
            verse: first.parse().ok()?,
        };
        let end = match last {
            None => start,
            Some(b) => match b.split_once(':') {
                Some((c, v)) => VerseRef {
                    book,
                    chapter: c.trim().parse().ok()?,
                    verse: v.trim().parse().ok()?,
                },
                None => VerseRef {
                    book,
                    chapter,
                    verse: b.parse().ok()?,
                },
            },
        };

        let mut label = reference_label(bible, start, end);
        if lxx {
            label.push_str(" LXX");
        }
        let range = (index.contains(start) && index.contains(end) && start <= end).then_some((start, end));
        out.push(Target { label, range });
    }
    Some(out)
}
