// ── Add your header block above ──

//! The in-memory shape of the Bible text.
//!
//! Everything Sojourner puts on a page, reads aloud, or links from is read out
//! of these types. They are produced once, by the USFM parser (`text::usfm`),
//! straight from the publisher's files in `assets/eng-web_usfm.zip`, and are
//! never edited afterwards.
//!
//! Two rules shaped this file:
//!
//! 1. Model the source, not the screen. USFM is a stream of *blocks*
//!    (paragraphs, poetry lines, headings), and verse numbers are *markers
//!    inside that stream*. One paragraph can hold several verses; one verse
//!    can run across several poetry lines. So a verse here is a marker in a
//!    block's content, not a box that owns text. Whoever needs "the text of
//!    John 3:16" walks from that marker to the next one.
//!
//! 2. Nothing is dropped. Every marker the publisher uses (the census is in
//!    assets/SOURCES.md) has a home in these types. When the parser meets a
//!    marker with no home here, that is a hard error, not a skip, so a future
//!    WEB update can never silently lose words.

/// The whole World English Bible, in the publisher's file order:
/// front matter, Old Testament, Deuterocanon, New Testament, glossary.
///
/// Display order (Deuterocanon shelved after the New Testament) is the app's
/// decision, made at view time. This struct keeps the publisher's order so
/// the data always matches the source.
#[derive(Debug)]
pub struct Bible {
    pub books: Vec<Book>,
}

/// Which shelf a book belongs on. Derived from the USFM book code (`\id`),
/// not the file name, so it survives the publisher renumbering files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `00-FRT`: the publisher's preface and notes. Shown in full, in its own
    /// place; not paged as scripture.
    FrontMatter,
    OldTestament,
    Deuterocanon,
    NewTestament,
    /// `106-GLO`: the translators' glossary of terms.
    Glossary,
}

/// One book. USFM also treats the front matter and the glossary as "books",
/// and so do we: they simply have no chapters, only `intro` blocks.
#[derive(Debug)]
pub struct Book {
    /// The publisher's file this book was read from, e.g.
    /// `20-PSAeng-web.usfm`. Kept so any question about a verse can be
    /// traced to a file and line in the source zip.
    pub file: String,
    /// Three-letter USFM code from `\id`: GEN, PSA, JHN, TOB, FRT, GLO…
    pub code: String,
    pub section: Section,
    pub names: BookNames,
    /// `\cl` given before the first chapter: a word to print in place of
    /// "Chapter" for every chapter of this book. The publisher uses it once,
    /// for the Psalms ("Psalm 23", not "Chapter 23").
    pub chapter_label: Option<String>,
    /// Blocks that come before the first `\c`: introductions, and the whole
    /// body of chapter-less books (front matter, glossary).
    pub intro: Vec<Block>,
    pub chapters: Vec<Chapter>,
}

/// The names the translators gave the book. Sojourner never invents names;
/// every string here came from the file.
#[derive(Debug, Default)]
pub struct BookNames {
    /// `\toc1`: long form, "The Good News According to John".
    pub long: String,
    /// `\toc2`: short form used in navigation, "John".
    pub short: String,
    /// `\toc3`: abbreviation, "Jhn".
    pub abbrev: String,
    /// `\h`: running-header text.
    pub header: String,
    /// `\mt1` / `\mt2` / `\mt3`: the title lines as printed on the book's
    /// first page, in reading order ("The Good News According to", "John").
    pub title_lines: Vec<String>,
}

#[derive(Debug)]
pub struct Chapter {
    /// From `\c`. This is the numbering the publisher uses in `\v` and `\fr`,
    /// i.e. English versification.
    pub number: u32,
    /// `\cl` given after this chapter's `\c`: a label for this one chapter.
    /// Allowed by USFM; the publisher's files don't currently use it (their
    /// one `\cl` is book-level, see `Book::chapter_label`).
    pub label: Option<String>,
    /// `\cp`: a "published" chapter number when it differs from `number`.
    /// Used once, for Psalm 151; kept so the page can show what the printed
    /// book shows.
    pub published_number: Option<String>,
    pub blocks: Vec<Block>,
}

/// One block-level unit: a paragraph, a line of poetry, a heading…
///
/// `content` is the flat sequence of inlines the publisher wrote, verse
/// markers included. See the module docs for why verses aren't containers.
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub content: Vec<Inline>,
}

/// Every block-level marker in the publisher's files, named after its USFM
/// marker so this code can always be mapped back to the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `\p`: ordinary prose paragraph, first line indented.
    Paragraph,
    /// `\m`: paragraph with no first-line indent (e.g. prose resuming after
    /// poetry).
    Flush,
    /// `\pi1`: indented paragraph (whole block set in from the margin).
    Indented,
    /// `\pc`: centered paragraph.
    Centered,
    /// `\mi`: indented paragraph with no first-line indent.
    IndentedFlush,
    /// `\nb`: "no break". The paragraph continues straight across a chapter
    /// boundary; render it as a continuation of the previous block.
    NoBreak,
    /// `\li1`: list item.
    ListItem,
    /// `\q1` / `\q2` / `\q3`: one line of poetry at indent level 1, 2 or 3.
    Poetry(u8),
    /// `\b`: a blank line between stanzas. Never has content.
    Blank,
    /// `\d`: a Psalm superscription ("A Psalm by David…"). This is scripture,
    /// not a heading: it is read aloud and paged with the text.
    Superscription,
    /// `\sp`: a speaker label (Song of Songs).
    Speaker,
    /// `\s1`: section heading.
    Heading,
    /// `\ms1`: major section heading.
    MajorHeading,
    /// `\is1`: introduction heading.
    IntroHeading,
    /// `\ip`: introduction paragraph.
    IntroParagraph,
    /// `\ili`: introduction list item.
    IntroListItem,
}

impl BlockKind {
    /// Does text in this kind of block belong to the verse whose number
    /// precedes it? True for the running text of scripture (prose and
    /// poetry); false for superscriptions, headings, speaker labels and
    /// introduction material, which sit *between* verses rather than inside
    /// one.
    pub fn is_verse_flow(self) -> bool {
        match self {
            BlockKind::Paragraph
            | BlockKind::Flush
            | BlockKind::Indented
            | BlockKind::Centered
            | BlockKind::IndentedFlush
            | BlockKind::NoBreak
            | BlockKind::ListItem
            | BlockKind::Poetry(_)
            | BlockKind::Blank => true,
            BlockKind::Superscription
            | BlockKind::Speaker
            | BlockKind::Heading
            | BlockKind::MajorHeading
            | BlockKind::IntroHeading
            | BlockKind::IntroParagraph
            | BlockKind::IntroListItem => false,
        }
    }
}

/// A run of content inside a block.
#[derive(Debug, Clone)]
pub enum Inline {
    /// `\v N` or `\v N-M`. A marker: the verse's text is whatever follows, up
    /// to the next `Verse` marker, possibly in a later block.
    Verse(VerseNumber),
    /// Text, with a style and the publisher's per-word tags.
    Text(TextRun),
    /// `\f … \f*`: a translator's footnote, anchored at this point.
    Footnote(Footnote),
    /// `\x … \x*`: one of the translators' own cross-references.
    CrossRef(CrossRef),
}

/// `\v 15` gives start 15, end 15. `\v 15-16` (six of these in the whole
/// Bible, all in the Deuterocanon) gives start 15, end 16: one run of text
/// carries both numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerseNumber {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
    /// The publisher's Strong's tags for the words in `text`, by byte range.
    /// Preserved, not displayed; see SOURCES.md for why.
    pub words: Vec<WordTag>,
}

/// How a text run is set. One enum serves main text and footnote text alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    Normal,
    /// `\wj … \wj*`: words of Jesus (red-letter).
    WordsOfJesus,
    /// `\qs Selah\qs*`: set right-aligned and italic at the end of a poetry
    /// line.
    Selah,
    /// `\k … \k*`: a glossary keyword.
    Keyword,
    /// `\bk … \bk*` and `\+bk … \+bk*`: the title of a book, quoted.
    BookName,
    /// `\+wh … \+wh*`: a Hebrew word cited in a footnote.
    Hebrew,
    /// `\fq`: footnote text quoting the main text being commented on.
    NoteQuote,
    /// `\fqa`: footnote text giving an alternate reading or translation.
    NoteAlternate,
    /// `\fl`: a footnote label.
    NoteLabel,
}

/// One tagged word: `\w heavens|strong="H8064"\w*`. `start..end` is the byte
/// range of the word within its `TextRun.text`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordTag {
    pub start: u32,
    pub end: u32,
    pub strong: StrongsId,
}

/// A Strong's concordance number: "H8064" is Hebrew 8064, "G1537" is Greek
/// 1537. Four bytes instead of a string, since there are 677,690 of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StrongsId {
    pub lang: StrongsLang,
    pub number: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrongsLang {
    Hebrew,
    Greek,
}

#[derive(Debug, Clone)]
pub struct Footnote {
    /// `\fr 8:37`: the chapter:verse the note belongs to, as the publisher
    /// wrote it.
    pub reference: String,
    /// The note's text as styled runs (`\ft`, `\fq`, `\fqa`, `\fl`, `\+wh`,
    /// `\+bk`).
    pub content: Vec<TextRun>,
}

#[derive(Debug, Clone)]
pub struct CrossRef {
    /// `\xo 1:23`: where the reference is anchored, as written.
    pub origin: String,
    /// `\xt Isaiah 7:14`: the target(s), exactly as the translators wrote
    /// them. Turning this into a `VerseRef` the app can jump to is a separate,
    /// checkable step; a target that can't be resolved stays here as text and
    /// is reported, never dropped.
    pub targets: String,
}

/// A pointer to one verse anywhere in the Bible. The trail, cross-references,
/// bookmarks and the TTS position all speak in these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VerseRef {
    /// Index into `Bible::books`.
    pub book: u8,
    pub chapter: u32,
    pub verse: u32,
}
