// ── Add your header block above ──

//! The page: a chapter typeset into sheets of a fixed size.
//!
//! A chapter becomes paragraphs of *pieces* — runs of text that each have
//! one look: a word or many in the body face, a verse number, a marker, the
//! small letters of "LORD" — and the paragraphs are dealt onto sheets by
//! measuring them. The measuring is done by the same paragraph engine the
//! window draws with (iced's, on cosmic-text), fed the very spans the
//! window will be fed, at the width the sheet gives them; so the lines it
//! counts are the lines that will be drawn. A paragraph that does not fit
//! the room left on a sheet is split between the lines the engine laid out,
//! with a printer's care: no single line left behind or carried over, and
//! no heading stranded at the foot of a page.
//!
//! Nothing here draws. The window (`main.rs`) turns a sheet's pieces into
//! spans with colours and links; this module decides what is on each
//! sheet, and `tests/pages.rs` proves that nothing is lost or doubled in
//! the dealing and that every sheet fits.

use std::collections::VecDeque;

use cosmic::iced::advanced::graphics::text::Paragraph as Typeset;
use cosmic::iced::advanced::text::{Alignment, Ellipsize, Paragraph as _, Shaping, Text as TextSpec, Wrapping};
use cosmic::iced::core::text::LineHeight;
use cosmic::iced::widget::text::{IntoFragment, Span};
use cosmic::iced::widget::span;
use cosmic::iced::{self, alignment, Font, Pixels, Size};

use crate::text::{Block, BlockKind, Book, Inline, TextStyle, VerseRef};

// ────────────────────────────────────────────────────────────────────────────
// The sheet
// ────────────────────────────────────────────────────────────────────────────

/// A serif face for the page. `Family::Serif` asks the system for its
/// default serif; on Pop!_OS that is usually DejaVu Serif or Noto Serif. A
/// bundled book face can replace this later without touching the layout.
pub const SERIF: Font = Font {
    family: iced::font::Family::Serif,
    ..Font::DEFAULT
};

pub const SERIF_ITALIC: Font = Font {
    family: iced::font::Family::Serif,
    style: iced::font::Style::Italic,
    ..Font::DEFAULT
};

pub const SERIF_BOLD: Font = Font {
    family: iced::font::Family::Serif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

/// The sheet: one page of the book, in logical pixels, whatever the window
/// is. The window is sized to show it whole; a smaller window scrolls.
pub const SHEET_WIDTH: f32 = 640.0;
pub const SHEET_HEIGHT: f32 = 860.0;

/// The sheet's margins. The running head and the folio sit in the top and
/// bottom ones.
pub const MARGIN_X: f32 = 44.0;
pub const MARGIN_Y: f32 = 52.0;

/// The text area of the sheet: 552 × 756, which at the default size is
/// exactly 28 lines of body text.
pub const TEXT_WIDTH: f32 = SHEET_WIDTH - 2.0 * MARGIN_X;
pub const TEXT_HEIGHT: f32 = SHEET_HEIGHT - 2.0 * MARGIN_Y;

/// Body text size to start with; the reader can change it within the
/// range. Every other size on the page is a share of it, and the air
/// between paragraphs scales with it too.
pub const DEFAULT_TEXT_SIZE: f32 = 18.0;
pub const SMALLEST_TEXT: f32 = 14.0;
pub const LARGEST_TEXT: f32 = 26.0;

/// Small capitals ("LORD"): the letters after the first, as a share of the
/// body size.
pub const SMALL_CAPS: f32 = 0.8;

/// Verse numbers and note markers, as a share of the body size.
pub const NUMBER: f32 = 11.0 / 18.0;

/// What a click on the page means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLink {
    /// A verse number: this verse and its cross-references.
    Verse(VerseRef),
    /// The n-th footnote marker inside a verse.
    Note(VerseRef, usize),
    /// The n-th translators' cross-reference marker inside a verse.
    Xref(VerseRef, usize),
}

/// "Psalm 23", "Genesis 1", or the name of a chapterless file ("Preface").
pub fn chapter_title(book: &Book, page: usize) -> String {
    match book.chapters.get(page) {
        Some(chapter) => {
            // "John 3", or "Psalm 23" where the publisher gave a chapter label.
            let label = book.chapter_label.as_deref().unwrap_or(&book.names.short);
            let number = chapter
                .published_number
                .clone()
                .unwrap_or_else(|| chapter.number.to_string());
            format!("{label} {number}")
        }
        None => book.names.short.clone(),
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Typesetting
// ────────────────────────────────────────────────────────────────────────────

/// One sheet of a chapter as typeset: the paragraphs (or parts of them) it
/// holds, and the verses among them.
#[derive(Debug, Clone)]
pub struct Leaf {
    pub pars: Vec<Par>,
    /// The first and last verse with words on this sheet.
    pub first_verse: Option<u32>,
    pub last_verse: Option<u32>,
}

/// A paragraph as it will be set: its shape, and its pieces.
#[derive(Debug, Clone)]
pub struct Par {
    pub shape: Shape,
    pub pieces: Vec<Piece>,
    /// Set on the rest of a paragraph begun on the sheet before.
    pub continued: bool,
}

/// What kind of paragraph, for its geometry: the publisher's block kinds,
/// and Sojourner's own furniture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Block(BlockKind),
    /// "Genesis 1": the chapter's label.
    ChapterLabel,
    /// A line of the book's title: the lead-in lines small and italic
    /// ("The First Book of Moses,"), the last large ("Genesis").
    TitleLine { last: bool },
    /// Air, this many pixels at the default text size.
    Air(f32),
}

/// A run of text with one look.
#[derive(Debug, Clone)]
pub struct Piece {
    pub text: String,
    pub face: Face,
    pub scale: Scale,
    pub ink: Ink,
    /// The verse this piece is part of, for the spoken highlight, when it is
    /// in the verse flow.
    pub verse: Option<u32>,
    pub link: Option<PageLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// The paragraph's own face.
    Body,
    Italic,
    /// The interface face: verse numbers, markers, glossary keywords.
    Sans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Body,
    /// The letters after the first of a word set in capitals.
    SmallCaps,
    /// Verse numbers and markers.
    Number,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// The sheet's ink.
    Body,
    /// Present but quieter: superscriptions, speaker labels.
    Muted,
    /// Words of Jesus.
    Red,
    /// Links: verse numbers and markers.
    Accent,
}

/// How a paragraph of a shape is set at a body size: face, size, line
/// height, alignment, the indent of its left edge, the air above and below
/// it, and its ink.
pub struct Geometry {
    pub font: Font,
    pub size: f32,
    pub line: f32,
    pub align: Alignment,
    pub indent: f32,
    pub above: f32,
    pub below: f32,
}

pub fn geometry(shape: Shape, body: f32) -> Geometry {
    // Air and indents were drawn for 18 px text and scale with it.
    let u = body / DEFAULT_TEXT_SIZE;
    let line = body * 1.5;
    let g = |font, size, line, align, indent, above, below| Geometry { font, size, line, align, indent, above, below };
    match shape {
        Shape::Block(kind) => match kind {
            // Prose, justified like a book. Paragraph spacing above.
            BlockKind::Paragraph | BlockKind::NoBreak | BlockKind::IntroParagraph => {
                g(SERIF, body, line, Alignment::Justified, 0.0, 8.0 * u, 0.0)
            }
            BlockKind::Flush | BlockKind::IndentedFlush => g(SERIF, body, line, Alignment::Justified, 0.0, 4.0 * u, 0.0),
            BlockKind::Indented | BlockKind::ListItem | BlockKind::IntroListItem => {
                g(SERIF, body, line, Alignment::Justified, 24.0 * u, 4.0 * u, 0.0)
            }
            BlockKind::Centered => g(SERIF, body, line, Alignment::Center, 0.0, 8.0 * u, 0.0),
            // Poetry: one line per block, indented by level, no paragraph
            // spacing so the lines read as verse.
            BlockKind::Poetry(level) => {
                let indent = (16.0 + 24.0 * f32::from(level.saturating_sub(1))) * u;
                g(SERIF, body, line, Alignment::Left, indent, 0.0, 0.0)
            }
            // A Psalm's superscription: scripture, set quietly in italic.
            BlockKind::Superscription => g(SERIF_ITALIC, body, line, Alignment::Left, 0.0, 6.0 * u, 10.0 * u),
            // Speaker labels (Song of Songs), section headings.
            BlockKind::Speaker => g(SERIF_ITALIC, body, line, Alignment::Left, 0.0, 10.0 * u, 2.0 * u),
            BlockKind::Heading | BlockKind::IntroHeading => g(SERIF_BOLD, body, line, Alignment::Left, 0.0, 14.0 * u, 4.0 * u),
            BlockKind::MajorHeading => {
                let size = body + 4.0 * u;
                g(SERIF_BOLD, size, size * 1.5, Alignment::Left, 0.0, 18.0 * u, 6.0 * u)
            }
            // A stanza break: just air.
            BlockKind::Blank => g(SERIF, body, line, Alignment::Left, 0.0, 12.0 * u, 0.0),
        },
        Shape::ChapterLabel => g(SERIF, 30.0 * u, 39.0 * u, Alignment::Left, 0.0, 0.0, 10.0 * u),
        Shape::TitleLine { last: true } => g(SERIF, 34.0 * u, 44.0 * u, Alignment::Left, 0.0, 0.0, 0.0),
        Shape::TitleLine { last: false } => g(SERIF_ITALIC, 16.0 * u, 24.0 * u, Alignment::Left, 0.0, 0.0, 0.0),
        Shape::Air(px) => g(SERIF, body, line, Alignment::Left, 0.0, px * u, 0.0),
    }
}

/// The ink a shape's words are set in unless a run says otherwise.
fn ink_of(shape: Shape) -> Ink {
    match shape {
        Shape::Block(BlockKind::Superscription | BlockKind::Speaker) | Shape::TitleLine { last: false } => Ink::Muted,
        _ => Ink::Body,
    }
}

/// Whether a paragraph of this shape may be split between two sheets.
/// Prose may; a heading, a label, a line of poetry or a superscription is
/// carried over whole.
pub fn splittable(shape: Shape) -> bool {
    matches!(
        shape,
        Shape::Block(
            BlockKind::Paragraph
                | BlockKind::NoBreak
                | BlockKind::IntroParagraph
                | BlockKind::Flush
                | BlockKind::IndentedFlush
                | BlockKind::Indented
                | BlockKind::ListItem
                | BlockKind::IntroListItem
                | BlockKind::Centered
        )
    )
}

/// Whether a paragraph of this shape is a heading, which is kept with
/// what follows it rather than left at the foot of a sheet.
pub fn is_heading(shape: Shape) -> bool {
    matches!(
        shape,
        Shape::Block(BlockKind::Heading | BlockKind::IntroHeading | BlockKind::MajorHeading | BlockKind::Speaker | BlockKind::Superscription)
            | Shape::ChapterLabel
            | Shape::TitleLine { .. }
    )
}

/// Whether a paragraph is only air: a stanza break, or Sojourner's own
/// spacing.
pub fn is_air(par: &Par) -> bool {
    matches!(par.shape, Shape::Air(_) | Shape::Block(BlockKind::Blank))
}

/// Which verse the typesetter is inside as it walks a chapter's blocks, so
/// each verse number, footnote marker and cross-reference marker can carry
/// the right link and every piece knows its verse. Footnotes are numbered
/// within their verse, in order.
pub struct Setter {
    book: u8,
    chapter: u32,
    verse: Option<u32>,
    notes: usize,
    xrefs: usize,
}

impl Setter {
    pub fn new(book: u8, chapter: u32) -> Self {
        Setter { book, chapter, verse: None, notes: 0, xrefs: 0 }
    }

    fn here(&self) -> Option<VerseRef> {
        self.verse.map(|verse| VerseRef { book: self.book, chapter: self.chapter, verse })
    }
}

/// A chapter (or a chapterless file) as paragraphs, in order.
pub fn typeset_chapter(book: &Book, book_ix: u8, page: usize) -> Vec<Par> {
    let mut pars = Vec::new();

    // The lead-in lines ("The First Book of Moses," / "Commonly Called")
    // are set small and italic; the name itself ("Genesis") upright and
    // large, the way a printed Bible does it.
    if page == 0 && !book.names.title_lines.is_empty() {
        for (i, line) in book.names.title_lines.iter().enumerate() {
            let last = i + 1 == book.names.title_lines.len();
            pars.push(Par::plain(Shape::TitleLine { last }, line));
        }
        pars.push(Par::air(if book.chapters.is_empty() { 18.0 } else { 26.0 }));
    }

    match book.chapters.get(page) {
        None => {
            for block in &book.intro {
                pars.push(set_block(block, None));
            }
        }
        Some(chapter) => {
            pars.push(Par::plain(Shape::ChapterLabel, &chapter_title(book, page)));
            let mut setter = Setter::new(book_ix, chapter.number);
            for block in &chapter.blocks {
                pars.push(set_block(block, Some(&mut setter)));
            }
        }
    }
    pars
}

impl Par {
    /// A paragraph of one run in the shape's own ink.
    pub fn plain(shape: Shape, text: &str) -> Par {
        Par {
            shape,
            pieces: vec![Piece {
                text: clean(text),
                face: Face::Body,
                scale: Scale::Body,
                ink: ink_of(shape),
                verse: None,
                link: None,
            }],
            continued: false,
        }
    }

    pub fn air(px: f32) -> Par {
        Par { shape: Shape::Air(px), pieces: Vec::new(), continued: false }
    }

    /// The pieces' text run together, as the paragraph engine sees it.
    pub fn text_len(&self) -> usize {
        self.pieces.iter().map(|p| p.text.len()).sum()
    }
}

/// One of the publisher's blocks as a paragraph. Links and verses only
/// make sense in the verse flow: a footnote in a superscription belongs to
/// no verse.
fn set_block(block: &Block, setter: Option<&mut Setter>) -> Par {
    let setter = if block.kind.is_verse_flow() { setter } else { None };
    set_content(Shape::Block(block.kind), &block.content, setter)
}

/// A run of inlines as a paragraph of pieces: small verse numbers set into
/// the words, footnote and cross-reference markers where the publisher
/// anchored them, words of Jesus in red, words in capitals as small
/// capitals. With a setter, the numbers and markers are links.
pub fn set_content(shape: Shape, content: &[Inline], mut setter: Option<&mut Setter>) -> Par {
    let ink = ink_of(shape);
    let mut pieces: Vec<Piece> = Vec::with_capacity(content.len() * 2);

    // A `\p` paragraph's first line is indented, as in print: an em space
    // before its first word.
    if shape == Shape::Block(BlockKind::Paragraph) {
        pieces.push(Piece {
            text: "\u{2003}".to_string(),
            face: Face::Body,
            scale: Scale::Body,
            ink,
            verse: None,
            link: None,
        });
    }

    for inline in content {
        match inline {
            Inline::Verse(n) => {
                let label = if n.end > n.start {
                    format!("{}-{}", n.start, n.end)
                } else {
                    n.start.to_string()
                };
                let (verse, link) = match setter.as_deref_mut() {
                    Some(s) => {
                        s.verse = Some(n.start);
                        s.notes = 0;
                        s.xrefs = 0;
                        (Some(n.start), s.here().map(PageLink::Verse))
                    }
                    None => (None, None),
                };
                // A verse number after words gets the space the print has
                // before it — a break opportunity, and part of the verse
                // just ended — and a no-break space after it keeps it with
                // its word.
                if pieces.last().is_some_and(|p| !p.text.ends_with(char::is_whitespace)) {
                    pieces.push(Piece {
                        text: " ".to_string(),
                        face: Face::Body,
                        scale: Scale::Body,
                        ink,
                        verse: pieces.last().and_then(|p| p.verse),
                        link: None,
                    });
                }
                pieces.push(Piece {
                    text: format!("{label}\u{A0}"),
                    face: Face::Sans,
                    scale: Scale::Number,
                    ink: Ink::Accent,
                    verse,
                    link,
                });
            }
            Inline::Text(run) => {
                let face = match run.style {
                    TextStyle::Selah | TextStyle::Hebrew | TextStyle::NoteQuote | TextStyle::NoteAlternate => Face::Italic,
                    TextStyle::Keyword => Face::Sans,
                    _ => Face::Body,
                };
                let ink = if run.style == TextStyle::WordsOfJesus { Ink::Red } else { ink };
                let verse = setter.as_deref().and_then(|s| s.verse);
                // The divine name is set in capitals in the text ("LORD",
                // "GOD"); a printed Bible sets it in small capitals. There
                // is no small-caps face, so the word's first letter keeps
                // the body size and the rest are set smaller.
                for (piece, small) in small_capitals(&run.text) {
                    if piece.is_empty() {
                        continue;
                    }
                    pieces.push(Piece {
                        text: clean(piece),
                        face,
                        scale: if small { Scale::SmallCaps } else { Scale::Body },
                        ink,
                        verse,
                        link: None,
                    });
                }
            }
            Inline::Footnote(_) => {
                let (verse, link) = match setter.as_deref_mut() {
                    Some(s) => {
                        let link = s.here().map(|at| PageLink::Note(at, s.notes));
                        if link.is_some() {
                            s.notes += 1;
                        }
                        (s.verse, link)
                    }
                    None => (None, None),
                };
                pieces.push(Piece { text: "†".to_string(), face: Face::Sans, scale: Scale::Number, ink: Ink::Accent, verse, link });
            }
            Inline::CrossRef(_) => {
                let (verse, link) = match setter.as_deref_mut() {
                    Some(s) => {
                        let link = s.here().map(|at| PageLink::Xref(at, s.xrefs));
                        if link.is_some() {
                            s.xrefs += 1;
                        }
                        (s.verse, link)
                    }
                    None => (None, None),
                };
                pieces.push(Piece { text: "‡".to_string(), face: Face::Sans, scale: Scale::Number, ink: Ink::Accent, verse, link });
            }
        }
    }
    Par { shape, pieces, continued: false }
}

/// Text as one paragraph: a line break inside a run (there should be none)
/// would start a new line in the engine and throw the measuring off.
fn clean(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// Small capitals, faked: a word set in capitals ("LORD", "GOD") is split
/// into its first letter, kept at body size, and the rest, set at
/// `SMALL_CAPS` of it — "L" + "ORD". Everything else passes through whole.
/// The pieces are slices of the run, in order; the flag says which are the
/// small ones.
pub fn small_capitals(text: &str) -> Vec<(&str, bool)> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < text.len() {
        // A run of letters.
        let word_end = text[i..].find(|c: char| !c.is_alphabetic()).map_or(text.len(), |n| i + n);
        if word_end > i && word_end - i >= 2 && text[i..word_end].chars().all(char::is_uppercase) {
            let first = text[i..].chars().next().unwrap().len_utf8();
            if i > start {
                pieces.push((&text[start..i], false));
            }
            pieces.push((&text[i..i + first], false));
            pieces.push((&text[i + first..word_end], true));
            start = word_end;
        }
        // Skip past this word (or this one non-letter character).
        i = if word_end > i { word_end } else { i + text[i..].chars().next().map_or(1, char::len_utf8) };
    }
    if start < text.len() {
        pieces.push((&text[start..], false));
    }
    pieces
}

/// A piece as a span with its geometry — face and size — and nothing else.
/// The measurer uses exactly this; the renderer adds colour and links,
/// which change no line.
pub fn shaped_span<'a>(text: impl IntoFragment<'a>, piece: &Piece, g: &Geometry) -> Span<'a, PageLink, Font> {
    let mut s = span(text).font(match piece.face {
        Face::Body => g.font,
        Face::Italic => SERIF_ITALIC,
        Face::Sans => Font::DEFAULT,
    });
    match piece.scale {
        Scale::Body => {}
        Scale::SmallCaps => s = s.size(g.size * SMALL_CAPS),
        Scale::Number => s = s.size(g.size * NUMBER),
    }
    s
}

/// The lines a paragraph sets into at the sheet's width: each line's height
/// and the byte offset, into the pieces' text run together, where it
/// starts. Measured by the renderer's own paragraph engine.
pub fn lines_of(par: &Par, g: &Geometry) -> Vec<(f32, usize)> {
    let spans: Vec<Span<'_, PageLink, Font>> = par.pieces.iter().map(|p| shaped_span(p.text.as_str(), p, g)).collect();
    let set = Typeset::with_spans(TextSpec {
        content: spans.as_slice(),
        bounds: Size::new(TEXT_WIDTH - g.indent, f32::INFINITY),
        size: Pixels(g.size),
        line_height: LineHeight::Absolute(Pixels(g.line)),
        font: g.font,
        align_x: g.align,
        align_y: alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::Word,
        ellipsize: Ellipsize::None,
    });
    set.buffer()
        .layout_runs()
        .map(|run| {
            // The line's first character in reading order (a line of mixed
            // direction lists its glyphs in visual order).
            let start = run.glyphs.iter().map(|glyph| glyph.start).min().unwrap_or(0);
            (run.line_height, start)
        })
        .collect()
}

/// Deal paragraphs onto sheets of `TEXT_HEIGHT`.
pub fn paginate(pars: Vec<Par>, body: f32) -> Vec<Leaf> {
    let mut leaves: Vec<Leaf> = Vec::new();
    let mut sheet: Vec<Par> = Vec::new();
    let mut used = 0.0f32;
    let mut queue: VecDeque<Par> = pars.into();

    // The air above a paragraph is dropped at the top of a sheet, so every
    // sheet starts at the same line.
    let above = |g: &Geometry, sheet: &[Par]| if sheet.is_empty() { 0.0 } else { g.above };
    // Closing a sheet: air at its foot (a stanza break whose next line went
    // over) is dropped, like air at the top.
    let close = |sheet: &mut Vec<Par>, leaves: &mut Vec<Leaf>, used: &mut f32| {
        while sheet.last().is_some_and(is_air) {
            sheet.pop();
        }
        if !sheet.is_empty() {
            leaves.push(Leaf::new(std::mem::take(sheet)));
        }
        *used = 0.0;
    };

    while let Some(par) = queue.pop_front() {
        let g = geometry(par.shape, body);

        if is_air(&par) {
            // Air: never at the top of a sheet, and never the reason to
            // start one.
            if sheet.is_empty() {
                continue;
            }
            if used + g.above <= TEXT_HEIGHT {
                used += g.above;
                sheet.push(par);
            }
            continue;
        }

        let lines = lines_of(&par, &g);
        let text_height: f32 = lines.iter().map(|(h, _)| h).sum();
        let whole = above(&g, &sheet) + text_height + g.below;
        let room = TEXT_HEIGHT - used;

        if whole <= room {
            // It fits. A heading, though, is only placed if the start of what
            // follows fits under it.
            if is_heading(par.shape) && !sheet.is_empty() {
                if let Some(next) = queue.iter().find(|p| !is_air(p)) {
                    let ng = geometry(next.shape, body);
                    let next_lines = lines_of(next, &ng);
                    let opening: f32 = next_lines.iter().take(2).map(|(h, _)| h).sum();
                    if whole + ng.above + opening > room {
                        close(&mut sheet, &mut leaves, &mut used);
                        queue.push_front(par);
                        continue;
                    }
                }
            }
            used += whole;
            sheet.push(par);
            continue;
        }

        // It doesn't fit. How many of its lines would?
        let mut fit = 0;
        let mut height = above(&g, &sheet);
        for (i, (h, _)) in lines.iter().enumerate() {
            if height + h <= room {
                height += h;
                fit = i + 1;
            } else {
                break;
            }
        }
        let total = lines.len();

        if splittable(par.shape) && total >= 2 {
            // Split between lines, leaving at least two on this sheet and
            // carrying at least two over.
            let mut keep = fit;
            if total - keep == 1 {
                keep -= 1;
            }
            if keep >= 2 && keep < total {
                let (head, tail) = split_par(par, lines[keep].1);
                sheet.push(head);
                close(&mut sheet, &mut leaves, &mut used);
                queue.push_front(tail);
                continue;
            }
        }

        if sheet.is_empty() {
            // Alone on a sheet and still too tall: it has to be split
            // wherever it can be, or taken whole if it is one line.
            if total >= 2 && fit >= 1 && fit < total {
                let (head, tail) = split_par(par, lines[fit].1);
                sheet.push(head);
                close(&mut sheet, &mut leaves, &mut used);
                queue.push_front(tail);
            } else {
                sheet.push(par);
                close(&mut sheet, &mut leaves, &mut used);
            }
            continue;
        }

        // Carry it over whole.
        close(&mut sheet, &mut leaves, &mut used);
        queue.push_front(par);
    }
    close(&mut sheet, &mut leaves, &mut used);

    if leaves.is_empty() {
        leaves.push(Leaf::new(Vec::new()));
    }
    leaves
}

/// A paragraph in two, at a byte offset into its text run together (a line
/// start, so always on a character boundary). The piece it falls inside is
/// cut; every piece keeps its look, verse and link.
fn split_par(par: Par, at: usize) -> (Par, Par) {
    let at = at.min(par.text_len());
    let mut head = Vec::new();
    let mut tail = Vec::new();
    let mut offset = 0;
    for piece in par.pieces {
        let (start, end) = (offset, offset + piece.text.len());
        offset = end;
        if end <= at {
            head.push(piece);
        } else if start >= at {
            tail.push(piece);
        } else {
            let cut = at - start;
            let mut first = piece.clone();
            first.text.truncate(cut);
            let mut rest = piece;
            rest.text = rest.text[cut..].to_string();
            head.push(first);
            tail.push(rest);
        }
    }
    (
        Par { shape: par.shape, pieces: head, continued: par.continued },
        Par { shape: par.shape, pieces: tail, continued: true },
    )
}

impl Leaf {
    pub fn new(pars: Vec<Par>) -> Leaf {
        let verses = pars.iter().flat_map(|par| par.pieces.iter()).filter_map(|piece| piece.verse);
        let first_verse = verses.clone().next();
        let last_verse = verses.last();
        Leaf { pars, first_verse, last_verse }
    }

    /// The first verse that begins on this sheet (its number is here), as
    /// opposed to one carried over from the sheet before.
    pub fn first_numbered(&self) -> Option<u32> {
        self.pars.iter().flat_map(|par| par.pieces.iter()).find_map(|piece| match piece.link {
            Some(PageLink::Verse(v)) => Some(v.verse),
            _ => None,
        })
    }
}


