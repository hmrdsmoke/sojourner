// ── Add your header block above ──

//! Proof for the lookup side: that finding a verse by reference returns the
//! same words as walking the chapter, that ranges walk in reading order, and
//! that the translators' own cross-reference targets resolve.

use sojourner::text::{self, *};
use sojourner::Library;

/// Every verse found by reference has exactly the words `plain_verses` gives
/// it — 38,058 lookups against the sequential walk.
#[test]
fn verse_lookup_matches_the_sequential_walk() {
    let lib = Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let mut checked = 0usize;
    for (b, book) in lib.bible.books.iter().enumerate() {
        for (chapter, number, plain) in text::plain_verses(book) {
            let at = VerseRef { book: b as u8, chapter, verse: number.start };
            let found = text::verse(&lib.bible, at)
                .unwrap_or_else(|| panic!("{} {chapter}:{} not found", book.code, number.start));
            assert_eq!(found.number, number, "{} {chapter}:{}", book.code, number.start);
            assert_eq!(found.plain(), plain, "{} {chapter}:{}", book.code, number.start);
            // The first piece begins with the verse's own marker.
            assert!(matches!(found.pieces.first().map(|(_, i)| &i[0]), Some(Inline::Verse(_))));
            checked += 1;
        }
    }
    assert_eq!(checked, 38_058);
}

/// Asking for the second number of a bridged verse finds the same text.
#[test]
fn bridged_verses_are_found_under_either_number() {
    let lib = Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let sir = lib.index.book("SIR").expect("Sirach");
    // Sirach 11:15-16 is one of the six bridged verses.
    let a = text::verse(&lib.bible, VerseRef { book: sir, chapter: 11, verse: 15 }).expect("15");
    let b = text::verse(&lib.bible, VerseRef { book: sir, chapter: 11, verse: 16 }).expect("16");
    assert_eq!(a.number, VerseNumber { start: 15, end: 16 });
    assert_eq!(a.plain(), b.plain());
}

#[test]
fn ranges_walk_in_reading_order() {
    let lib = Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let chr2 = lib.index.book("2CH").expect("2 Chronicles");
    let ezr = lib.index.book("EZR").expect("Ezra");
    let start = VerseRef { book: chr2, chapter: 36, verse: 22 };
    let end = VerseRef { book: ezr, chapter: 1, verse: 3 };
    let walk = lib.index.between(start, end);
    // 2 Chronicles 36:22-23, then Ezra 1:1-3.
    assert_eq!(walk.len(), 5);
    assert_eq!(walk[0], start);
    assert_eq!(walk[4], end);
    assert_eq!(text::reference_label(&lib.bible, start, end), "2 Chronicles 36:22 – Ezra 1:3");

    let jhn = lib.index.book("JHN").expect("John");
    let v = VerseRef { book: jhn, chapter: 3, verse: 16 };
    assert_eq!(text::reference_label(&lib.bible, v, v), "John 3:16");
    let psa = lib.index.book("PSA").expect("Psalms");
    let p = VerseRef { book: psa, chapter: 23, verse: 1 };
    assert_eq!(text::reference_label(&lib.bible, p, VerseRef { verse: 3, ..p }), "Psalm 23:1–3");
}

/// The translators' 363 `\x` notes: every target they wrote resolves to
/// verses that exist, except the ones listed here, which are pinned so a
/// change in either direction is noticed.
#[test]
fn translators_cross_references_resolve() {
    let lib = Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let mut notes = 0usize;
    let mut targets = 0usize;
    let mut unresolved: Vec<String> = Vec::new();

    for book in &lib.bible.books {
        for chapter in &book.chapters {
            for block in &chapter.blocks {
                for inline in &block.content {
                    let Inline::CrossRef(xref) = inline else { continue };
                    notes += 1;
                    for target in text::parse_targets(&lib.bible, &lib.index, &xref.targets) {
                        targets += 1;
                        if target.range.is_none() {
                            unresolved.push(format!("{} {}: {:?} → {:?}", book.code, xref.origin, xref.targets, target.label));
                        }
                    }
                }
            }
        }
    }

    eprintln!("{notes} notes, {targets} targets, {} unresolved", unresolved.len());
    for u in &unresolved {
        eprintln!("  {u}");
    }
    assert_eq!(notes, 363);
    assert!(unresolved.is_empty(), "{} target(s) did not resolve (listed above)", unresolved.len());
}
