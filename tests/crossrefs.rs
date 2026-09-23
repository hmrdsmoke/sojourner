// ── Add your header block above ──

//! The proof for the cross-reference web (assets/cross-references.zip,
//! openbible.info, CC BY 4.0): that every one of its 344,799 references
//! points at a verse that exists in our WEB text, and that the only
//! renumbering applied is the documented one.

use sojourner::crossrefs::CrossRefs;
use sojourner::text::{self, VerseIndex, VerseRef};

fn load() -> (text::Bible, VerseIndex, CrossRefs) {
    let bible = text::usfm::load_bundled().unwrap_or_else(|e| panic!("parse failed: {e}"));
    let index = VerseIndex::new(&bible);
    let refs = CrossRefs::load_bundled(&index).unwrap_or_else(|e| panic!("cross-references failed: {e}"));
    (bible, index, refs)
}

#[test]
fn every_reference_resolves() {
    let (_, index, refs) = load();

    // 38,058 verse markers, plus the extra numbers covered by the six
    // bridged verses (15-16, 15-16, 18-19, 19-27, 28-29, 9-10 → +13).
    assert_eq!(index.len(), 38_071, "distinct verse numbers in the Bible");

    assert_eq!(refs.snapshot, "2026-09-21", "snapshot date in the file header");
    assert_eq!(refs.resolved + refs.unresolved.len(), 344_799, "lines in the file");
    for u in &refs.unresolved {
        eprintln!("  unresolved line {}: {} -> {} [{}]", u.line, u.from, u.to, u.reason);
    }
    assert!(refs.unresolved.is_empty(), "{} reference(s) did not resolve (listed above)", refs.unresolved.len());

    // The only renumbering: Romans 16:25–27 → 14:24–26 and 3 John 15 → 14,
    // wherever they occur as a source or a target.
    assert_eq!(refs.renumbered, 226, "renumbered verse references");
    assert_eq!(refs.source_count(), 29_363, "verses with outgoing links");
}

#[test]
fn links_are_well_formed_and_ordered() {
    let (bible, index, refs) = load();
    let mut links = 0usize;
    let mut cross_book = 0usize;

    for book in &bible.books {
        let Some(b) = index.book(&book.code) else { continue };
        for chapter in &book.chapters {
            for verse in 1..=200u32 {
                let source = VerseRef { book: b, chapter: chapter.number, verse };
                let out = refs.from(source);
                if out.is_empty() {
                    continue;
                }
                assert!(index.contains(source), "{} {}:{verse}: links from a verse that doesn't exist", book.code, chapter.number);
                let mut last_votes = i32::MAX;
                for link in out {
                    links += 1;
                    assert!(index.contains(link.target), "target missing");
                    assert!(index.contains(link.target_end), "target end missing");
                    assert!(link.target <= link.target_end, "range runs backwards");
                    assert!(link.votes <= last_votes, "links not sorted most-voted first");
                    last_votes = link.votes;
                    if link.target.book != link.target_end.book {
                        cross_book += 1;
                    }
                }
            }
        }
    }

    assert_eq!(links, 344_799, "links reachable through CrossRefs::from");
    // Ranges that run from the end of one book into the next, e.g.
    // 2 Chronicles 36:22 – Ezra 1:3.
    assert_eq!(cross_book, 18, "ranges that cross a book boundary");
}

#[test]
fn genesis_one_one_reads_like_a_study_bible() {
    let (_, index, refs) = load();
    let genesis = index.book("GEN").expect("Genesis");
    let jhn = index.book("JHN").expect("John");
    let heb = index.book("HEB").expect("Hebrews");

    let out = refs.from(VerseRef { book: genesis, chapter: 1, verse: 1 });
    assert!(!out.is_empty(), "Genesis 1:1 has cross-references");

    // The most-voted reference for "In the beginning God created" is
    // "In the beginning was the Word" — John 1:1–3.
    let top = out[0];
    assert_eq!(top.target, VerseRef { book: jhn, chapter: 1, verse: 1 });
    assert_eq!(top.target_end, VerseRef { book: jhn, chapter: 1, verse: 3 });
    assert_eq!(top.votes, 379);
    assert_eq!(out[1].target, VerseRef { book: heb, chapter: 11, verse: 3 });
}

/// The renumbering works in both directions: the doxology the set files
/// under Romans 16:26 must show up as links *from* WEB Romans 14:25.
#[test]
fn romans_doxology_is_renumbered_both_ways() {
    let (_, index, refs) = load();
    let rom = index.book("ROM").expect("Romans");
    let genesis = index.book("GEN").expect("Genesis");

    let from_doxology = refs.from(VerseRef { book: rom, chapter: 14, verse: 25 });
    assert!(!from_doxology.is_empty(), "WEB Romans 14:25 (= set's 16:26) has outgoing links");
    // The set's line "Rom.16.26 -> Gen.21.33" lands here.
    assert!(
        from_doxology.iter().any(|l| l.target == VerseRef { book: genesis, chapter: 21, verse: 33 }),
        "Romans 14:25 → Genesis 21:33"
    );
    // And nothing points at the WEB's empty 16:26 (it doesn't exist).
    assert!(!index.contains(VerseRef { book: rom, chapter: 16, verse: 26 }));
}
