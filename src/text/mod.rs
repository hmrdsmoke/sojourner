// ── Add your header block above ──

//! The Bible text: its in-memory model (`model`), the parser that builds it
//! from the publisher's USFM files (`usfm`), and a few read-only helpers
//! that walk the model.

pub mod index;
pub mod locate;
pub mod model;
pub mod usfm;

pub use index::VerseIndex;
pub use locate::{parse_targets, reference_label, verse, Target, VerseText};
pub use model::*;

/// The plain text of every verse in a book, in order.
///
/// "Plain" means the text runs only: no footnotes, no cross-references, no
/// styling. Where a verse continues into a later block (poetry lines, a
/// paragraph that runs on), the pieces are joined with a single space.
/// Text in blocks that aren't part of the verse flow (superscriptions,
/// headings, speaker labels) is not included in any verse.
///
/// This is what the witness test compares against the independent JSON, and
/// what the reader will hand to the TTS engine.
pub fn plain_verses(book: &Book) -> Vec<(u32, VerseNumber, String)> {
    let mut out: Vec<(u32, VerseNumber, String)> = Vec::new();

    for chapter in &book.chapters {
        // Index into `out` of the verse currently being accumulated. Reset
        // at each chapter so a superscription or heading at the top of a
        // chapter can never be glued onto the previous chapter's last verse.
        let mut current: Option<usize> = None;

        for block in &chapter.blocks {
            if !block.kind.is_verse_flow() {
                continue;
            }
            // First text appended from this block gets a separating space.
            let mut need_space = true;

            for inline in &block.content {
                match inline {
                    Inline::Verse(number) => {
                        out.push((chapter.number, *number, String::new()));
                        current = Some(out.len() - 1);
                        need_space = false;
                    }
                    Inline::Text(run) => {
                        if let Some(i) = current {
                            let text = &mut out[i].2;
                            if need_space && !text.is_empty() && !text.ends_with(' ') {
                                text.push(' ');
                            }
                            text.push_str(&run.text);
                            need_space = false;
                        }
                    }
                    Inline::Footnote(_) | Inline::CrossRef(_) => {}
                }
            }
        }
    }

    out
}
