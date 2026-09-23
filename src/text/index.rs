// ── Add your header block above ──

//! A lookup table over the parsed Bible: which books exist (by code), and
//! which (book, chapter, verse) triples exist. Anything that points *into*
//! the text — a cross-reference, a bookmark, the trail — is checked against
//! this before it is trusted.

use std::collections::{HashMap, HashSet};

use super::model::*;

#[derive(Debug)]
pub struct VerseIndex {
    /// USFM code ("GEN", "1SA", "REV") → index into `Bible::books`.
    books_by_code: HashMap<String, u8>,
    /// Every verse number that exists. A bridged verse (`\v 15-16`) puts
    /// both 15 and 16 here, since a reference to either is a reference to
    /// that text.
    verses: HashSet<VerseRef>,
    /// The same verses in reading order, for walking a range.
    ordered: Vec<VerseRef>,
}

impl VerseIndex {
    pub fn new(bible: &Bible) -> Self {
        let mut books_by_code = HashMap::new();
        let mut verses = HashSet::new();

        for (i, book) in bible.books.iter().enumerate() {
            let book_ix = u8::try_from(i).expect("fewer than 256 books");
            books_by_code.insert(book.code.clone(), book_ix);
            for chapter in &book.chapters {
                for block in &chapter.blocks {
                    for inline in &block.content {
                        if let Inline::Verse(n) = inline {
                            for verse in n.start..=n.end {
                                verses.insert(VerseRef {
                                    book: book_ix,
                                    chapter: chapter.number,
                                    verse,
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut ordered: Vec<VerseRef> = verses.iter().copied().collect();
        ordered.sort();
        VerseIndex { books_by_code, verses, ordered }
    }

    /// Every verse from `start` to `end` inclusive, in reading order — the
    /// verses a cross-reference range like "2 Chronicles 36:22 – Ezra 1:3"
    /// covers. Empty if the range runs backwards.
    pub fn between(&self, start: VerseRef, end: VerseRef) -> &[VerseRef] {
        let from = self.ordered.partition_point(|v| *v < start);
        let to = self.ordered.partition_point(|v| *v <= end);
        if from <= to { &self.ordered[from..to] } else { &[] }
    }

    /// The book index for a USFM code, if the Bible has that book.
    pub fn book(&self, code: &str) -> Option<u8> {
        self.books_by_code.get(code).copied()
    }

    /// Does this exact verse exist in the text?
    pub fn contains(&self, verse: VerseRef) -> bool {
        self.verses.contains(&verse)
    }

    /// Number of distinct verse numbers in the whole Bible (bridged verses
    /// counted once per number).
    pub fn len(&self) -> usize {
        self.verses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.verses.is_empty()
    }
}
