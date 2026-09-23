// ── Add your header block above ──

//! The pages: proof that dealing a chapter onto sheets loses nothing,
//! doubles nothing, and fills no sheet past its bottom.
//!
//! Typesetting measures text with the real paragraph engine and the book
//! face compiled into the binary, so every machine sets the same sheets
//! and the counts here are exact, like every other count in these tests.
//! Every chapter is checked by `every_chapter_deals_onto_sheets_whole`,
//! which takes a few minutes in a debug build and so is ignored by
//! default; `cargo test` runs `a_sample_of_chapters_deals_onto_sheets_whole`
//! over every seventh chapter and the ones most likely to break. Run the
//! whole thing with `cargo test --test pages -- --ignored`.

use std::time::Instant;

use sha2::{Digest, Sha256};
use sojourner::page::{self, geometry, is_air, paginate, typeset_chapter, Leaf, PageLink, Par, BOOK_FACE_FILES, DEFAULT_TEXT_SIZE, TEXT_HEIGHT};
use sojourner::text::Book;
use sojourner::Library;

/// The text of paragraphs' pieces, run together.
fn text_of<'a>(pars: impl Iterator<Item = &'a Par>) -> String {
    pars.flat_map(|p| p.pieces.iter()).map(|p| p.text.as_str()).collect()
}

/// The verse numbers set as links, in order.
fn numbers_of<'a>(pars: impl Iterator<Item = &'a Par>) -> Vec<u32> {
    pars.flat_map(|p| p.pieces.iter())
        .filter_map(|p| match p.link {
            Some(PageLink::Verse(v)) => Some(v.verse),
            _ => None,
        })
        .collect()
}

/// A sheet's height as the window will draw it: no air above the first
/// paragraph, then each paragraph's air, lines and air below.
fn height_of(leaf: &Leaf, body: f32) -> f32 {
    let mut height = 0.0;
    for (n, par) in leaf.pars.iter().enumerate() {
        let g = geometry(par.shape, body);
        let above = if n == 0 { 0.0 } else { g.above };
        if is_air(par) {
            height += above;
            continue;
        }
        let lines: f32 = page::lines_of(par, &g).iter().map(|(h, _)| h).sum();
        height += above + lines + g.below;
    }
    height
}

fn line_count(par: &Par, body: f32) -> usize {
    page::lines_of(par, &geometry(par.shape, body)).len()
}

#[derive(Default)]
struct Tally {
    chapters: usize,
    sheets: usize,
    splits: usize,
    longest: (usize, String),
}

/// Typeset one chapter and check every invariant on its sheets.
fn check(book: &Book, book_ix: u8, page: usize, body: f32, tally: &mut Tally) {
    let pars = typeset_chapter(book, book_ix, page);
    let leaves = paginate(pars.clone(), body);
    let title = page::chapter_title(book, page);
    tally.chapters += 1;
    tally.sheets += leaves.len();
    if leaves.len() > tally.longest.0 {
        tally.longest = (leaves.len(), title.clone());
    }

    // Nothing lost, nothing doubled: the sheets' text, run together, is the
    // chapter's text run together, and every verse number appears once, in
    // order.
    let dealt = || leaves.iter().flat_map(|l| l.pars.iter());
    assert_eq!(text_of(dealt()), text_of(pars.iter()), "{title}: the sheets do not add up to the chapter");
    assert_eq!(numbers_of(dealt()), numbers_of(pars.iter()), "{title}: verse numbers");

    for (n, leaf) in leaves.iter().enumerate() {
        let here = format!("{title}, sheet {} of {}", n + 1, leaves.len());
        // No sheet is empty, and none begins or ends with air.
        assert!(!leaf.pars.is_empty(), "{here}: empty");
        assert!(!is_air(&leaf.pars[0]), "{here}: begins with air");
        assert!(!is_air(leaf.pars.last().unwrap()), "{here}: ends with air");
        // Every sheet fits its text area.
        let height = height_of(leaf, body);
        assert!(height <= TEXT_HEIGHT + 0.01, "{here}: {height} px of text on a {TEXT_HEIGHT} px sheet");
        // The verses noted on the sheet are the verses on it.
        let verses: Vec<u32> = leaf.pars.iter().flat_map(|p| p.pieces.iter()).filter_map(|p| p.verse).collect();
        assert_eq!(leaf.first_verse, verses.first().copied(), "{here}: first verse");
        assert_eq!(leaf.last_verse, verses.last().copied(), "{here}: last verse");

        let last = leaf.pars.last().unwrap();
        if n + 1 < leaves.len() {
            // No heading stranded at the foot of a sheet.
            assert!(!page::is_heading(last.shape), "{here}: ends with a heading");
        }
        // A paragraph split between sheets is prose, keeps at least two
        // lines on each side (unless it had a sheet to itself and still
        // overflowed it), and is cut where a line was broken: between words.
        if let Some(tail) = leaves.get(n + 1).map(|next| &next.pars[0]).filter(|tail| tail.continued) {
            tally.splits += 1;
            assert!(page::splittable(last.shape) || leaf.pars.len() == 1, "{here}: split a {:?}", last.shape);
            if leaf.pars.len() > 1 {
                assert!(line_count(last, body) >= 2, "{here}: one line left behind");
                assert!(line_count(tail, body) >= 2, "{here}: one line carried over");
            }
            let head_text = text_of(std::iter::once(last));
            let tail_text = text_of(std::iter::once(tail));
            let (before, after) = (head_text.chars().last().unwrap(), tail_text.chars().next().unwrap());
            assert!(
                before.is_whitespace() || after.is_whitespace() || !before.is_alphanumeric() || !after.is_alphanumeric(),
                "{here}: split inside a word: …{before:?} | {after:?}…"
            );
        }
    }
}

fn report(what: &str, tally: &Tally, started: Instant) {
    eprintln!(
        "{what}: {} chapters typeset into {} sheets ({} paragraphs split) in {:.1} s; the longest is {} at {} sheets",
        tally.chapters,
        tally.sheets,
        tally.splits,
        started.elapsed().as_secs_f32(),
        tally.longest.1,
        tally.longest.0
    );
}

#[test]
fn a_sample_of_chapters_deals_onto_sheets_whole() {
    let lib = Library::load_bundled().expect("the bundled text");
    let started = Instant::now();
    let mut tally = Tally::default();
    // The likely breakers: the chapterless files (long prose), the longest
    // chapter, the shortest, a bridged-verse chapter, the words of Jesus,
    // the renumbered doxology.
    let always = ["FRT", "GLO", "PSA", "ROM", "EST", "MAT", "SIR"];
    let mut n = 0usize;
    for (i, book) in lib.bible.books.iter().enumerate() {
        for page in 0..book.chapters.len().max(1) {
            n += 1;
            let wanted = n % 7 == 0
                || (always.contains(&book.code.as_str())
                    && (book.chapters.is_empty() || matches!(book.chapters[page].number, 1 | 14 | 16 | 117 | 119 | 151)));
            if wanted {
                check(book, i as u8, page, DEFAULT_TEXT_SIZE, &mut tally);
            }
        }
    }
    report("sample", &tally, started);
    assert_eq!(tally.chapters, 216);
    assert_eq!((tally.sheets, tally.splits), (723, 261), "the sample's sheets and splits at {DEFAULT_TEXT_SIZE} px");
    assert_eq!(tally.longest, (16, "Psalm 119".to_string()));
}

#[test]
#[ignore = "a few minutes in a debug build: cargo test --test pages -- --ignored"]
fn every_chapter_deals_onto_sheets_whole() {
    let lib = Library::load_bundled().expect("the bundled text");
    let started = Instant::now();
    let mut tally = Tally::default();
    for (i, book) in lib.bible.books.iter().enumerate() {
        for page in 0..book.chapters.len().max(1) {
            check(book, i as u8, page, DEFAULT_TEXT_SIZE, &mut tally);
        }
    }
    report("everything", &tally, started);
    assert_eq!(tally.chapters, 1_402 + 2, "every chapter, the preface and the glossary");
    assert_eq!((tally.sheets, tally.splits), (4_653, 1_725), "the whole book's sheets and splits at {DEFAULT_TEXT_SIZE} px");
    assert_eq!(tally.longest, (16, "Psalm 119".to_string()));
}

#[test]
fn a_larger_text_takes_more_sheets() {
    let lib = Library::load_bundled().expect("the bundled text");
    let genesis = lib.bible.books.iter().position(|b| b.code == "GEN").unwrap();
    let book = &lib.bible.books[genesis];
    let mut tally = Tally::default();
    for body in [14.0, 22.0, 26.0] {
        check(book, genesis as u8, 0, body, &mut tally);
    }
    let at = |body: f32| paginate(typeset_chapter(book, genesis as u8, 0), body).len();
    let (small, normal, large) = (at(14.0), at(DEFAULT_TEXT_SIZE), at(26.0));
    eprintln!("Genesis 1: {small} sheets at 14 px, {normal} at {DEFAULT_TEXT_SIZE} px, {large} at 26 px");
    assert_eq!((small, normal, large), (2, 3, 5));
}

#[test]
fn the_pieces_carry_their_verses_and_links() {
    let lib = Library::load_bundled().expect("the bundled text");
    let genesis = lib.bible.books.iter().position(|b| b.code == "GEN").unwrap();
    let pars = typeset_chapter(&lib.bible.books[genesis], genesis as u8, 0);
    let pieces: Vec<_> = pars.iter().flat_map(|p| p.pieces.iter()).collect();
    // The chapter's 31 verse numbers, each a link to its verse.
    assert_eq!(numbers_of(pars.iter()), (1..=31).collect::<Vec<u32>>());
    // Every word of verse 1 knows it is verse 1; the footnote marker after
    // "God" is the first note of verse 1.
    let text_of_1: String = pieces.iter().filter(|p| p.verse == Some(1) && p.link.is_none()).map(|p| p.text.as_str()).collect();
    assert_eq!(text_of_1.trim(), "In the beginning, God created the heavens and the earth.");
    let note = pieces.iter().find(|p| matches!(p.link, Some(PageLink::Note(_, _)))).unwrap();
    assert_eq!(note.text, "†");
    assert!(matches!(note.link, Some(PageLink::Note(v, 0)) if v.verse == 1 && v.chapter == 1));
    // A space before a verse number that follows words, none before the
    // first, and a no-break space after every one.
    let two = pieces.iter().position(|p| matches!(p.link, Some(PageLink::Verse(v)) if v.verse == 2)).unwrap();
    assert_eq!(pieces[two - 1].text, " ");
    assert_eq!(pieces[two - 1].verse, Some(1), "the space belongs to the verse just ended");
    assert_eq!(pieces[two].text, "2\u{A0}");
    let one = pieces.iter().position(|p| matches!(p.link, Some(PageLink::Verse(v)) if v.verse == 1)).unwrap();
    assert_eq!(pieces[one - 1].text, "\u{2003}", "the paragraph's first-line indent");
}

/// The book face compiled in is SIL's release of it, file for file: the
/// hashes recorded in assets/SOURCES.md, section 6.
#[test]
fn the_book_face_is_the_release_it_says() {
    let recorded = [
        ("GentiumBookPlus-Regular.ttf", "298d3e2d2cdf0460d27151de50e2a5764d4de4921f5a6fc254ed5aeda6890f1e"),
        ("GentiumBookPlus-Italic.ttf", "ab3d2755ad7e43ca680d4b527cc126a0099fb19d659464b5ea6e1f8b537828ad"),
        ("GentiumBookPlus-Bold.ttf", "bad0c69e4452a2c34d66754ad56a0f0c12b043cb40c44216337439eeae500831"),
    ];
    for ((name, bytes), (recorded_name, hash)) in BOOK_FACE_FILES.iter().zip(recorded) {
        assert_eq!(*name, recorded_name);
        let digest: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(digest, hash, "{name} is not the file SOURCES.md records");
    }
}
