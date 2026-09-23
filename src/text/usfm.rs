// ── Add your header block above ──

//! USFM parser for the World English Bible as shipped by eBible.org.
//!
//! This is not a general USFM parser. It knows exactly the markers the
//! publisher's files use (the census is in assets/SOURCES.md) and refuses
//! everything else. That is deliberate: a general parser would *skip* what it
//! doesn't understand, and skipping is how words go missing. Here an unknown
//! marker is a hard error carrying the file and line number, so a future WEB
//! release with a new marker breaks the build instead of the text.
//!
//! How the files are shaped (all 83 of them, checked):
//!   * every line starts with a marker; there are no continuation lines and
//!     no blank lines;
//!   * a *block* marker (`\p`, `\q1`, `\d`, …) starts a new block, and the
//!     rest of that line is the block's first content;
//!   * a `\v` line adds to the current block;
//!   * character markers (`\w`, `\wj`, `\f`, `\x`, …) appear inline.
//!
//! Whitespace: runs of whitespace collapse to one space (the files use double
//! spaces after sentences), and leading space is dropped at the start of a
//! block and right after a verse number. Otherwise spacing is kept as
//! written, so concatenating a block's text runs reproduces the publisher's
//! spacing around footnote markers and punctuation.

use std::io::Read;

use super::model::*;

/// The publisher's zip, compiled into the binary (3.1 MB). Its SHA-256 is
/// recorded in assets/SOURCES.md.
pub const BUNDLED_ZIP: &[u8] = include_bytes!("../../assets/engwebu_usfm.zip");

#[derive(Debug)]
pub enum Error {
    /// The zip could not be opened, or an entry could not be read.
    Zip(String),
    /// A marker this parser has no home for. See the module docs.
    UnknownMarker {
        file: String,
        line: usize,
        marker: String,
    },
    /// Something structurally wrong: a `\c` without a number, a closing
    /// marker with nothing open, a word tag without `strong=`, and so on.
    Malformed {
        file: String,
        line: usize,
        what: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Zip(e) => write!(f, "zip: {e}"),
            Error::UnknownMarker { file, line, marker } => {
                write!(f, "{file}:{line}: unknown USFM marker \\{marker}")
            }
            Error::Malformed { file, line, what } => write!(f, "{file}:{line}: {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// Parse the zip compiled into the binary.
pub fn load_bundled() -> Result<Bible, Error> {
    parse_zip(BUNDLED_ZIP)
}

/// Parse every `*.usfm` entry in a zip, in the publisher's numeric file
/// order (00-FRT first, 106-GLO last; a plain string sort would put 106
/// between 10 and 11).
pub fn parse_zip(bytes: &[u8]) -> Result<Bible, Error> {
    let zip_err = |e: zip::result::ZipError| Error::Zip(e.to_string());
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(zip_err)?;

    let mut entries: Vec<(u32, String)> = Vec::new();
    for i in 0..archive.len() {
        let name = archive.by_index(i).map_err(zip_err)?.name().to_string();
        if let Some(order) = file_order(&name) {
            entries.push((order, name));
        }
    }
    entries.sort();

    let mut books = Vec::with_capacity(entries.len());
    for (_, name) in entries {
        let mut text = String::new();
        archive
            .by_name(&name)
            .map_err(zip_err)?
            .read_to_string(&mut text)
            .map_err(|e| Error::Zip(format!("{name}: {e}")))?;
        books.push(parse_book(&name, &text)?);
    }
    Ok(Bible { books })
}

/// `20-PSAengwebu.usfm` → `Some(20)`. Anything that isn't a `.usfm` file
/// (copr.htm, keys.asc, the stylesheet) → `None`.
fn file_order(name: &str) -> Option<u32> {
    let base = name.rsplit('/').next()?;
    if !base.ends_with(".usfm") {
        return None;
    }
    let digits: String = base.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Parse one book file. `file` is only used in error messages.
pub fn parse_book(file: &str, text: &str) -> Result<Book, Error> {
    // Haiola writes a UTF-8 byte-order mark at the top of each file.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    let mut parser = Parser::new(file);
    for (index, raw) in text.lines().enumerate() {
        parser.line = index + 1;
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        parser.line_of(line)?;
    }
    parser.finish()
}

// ────────────────────────────────────────────────────────────────────────────
// The parser proper
// ────────────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    file: &'a str,
    line: usize,

    code: Option<String>,
    section: Option<Section>,
    names: BookNames,
    chapter_label: Option<String>,
    intro: Vec<Block>,
    chapters: Vec<Chapter>,

    /// The chapter being built. `None` until the first `\c`.
    chapter: Option<Chapter>,
    /// The block being built. `None` between a `\c` and its first block.
    block: Option<Block>,
    /// Open character styles (`\wj`, `\qs`, …), innermost last. Kept across
    /// lines and blocks, because the publisher lets `\wj` run on across
    /// paragraph breaks.
    styles: Vec<TextStyle>,
    /// True at the start of a block and right after a verse number: leading
    /// whitespace is dropped until the next real character.
    at_start: bool,
}

impl<'a> Parser<'a> {
    fn new(file: &'a str) -> Self {
        Parser {
            file,
            line: 0,
            code: None,
            section: None,
            names: BookNames::default(),
            chapter_label: None,
            intro: Vec::new(),
            chapters: Vec::new(),
            chapter: None,
            block: None,
            styles: Vec::new(),
            at_start: true,
        }
    }

    fn malformed(&self, what: impl Into<String>) -> Error {
        Error::Malformed {
            file: self.file.to_string(),
            line: self.line,
            what: what.into(),
        }
    }

    fn unknown(&self, marker: &str) -> Error {
        Error::UnknownMarker {
            file: self.file.to_string(),
            line: self.line,
            marker: marker.to_string(),
        }
    }

    /// One line of the file. Every line starts with a marker; which marker
    /// decides whether this is book metadata, a chapter start, a new block,
    /// or content for the current block.
    fn line_of(&mut self, line: &str) -> Result<(), Error> {
        let marker = read_marker(line).ok_or_else(|| self.malformed("line does not start with a marker"))?;
        let rest = &line[marker.len..];

        match marker.name {
            "id" => {
                let code = rest
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| self.malformed("\\id without a book code"))?;
                self.section = Some(
                    section_for(code).ok_or_else(|| self.malformed(format!("unknown book code {code}")))?,
                );
                self.code = Some(code.to_string());
            }
            // Character encoding declaration. Always UTF-8 in these files,
            // and Rust strings already are.
            "ide" => {}
            "h" => self.names.header = rest.trim().to_string(),
            "toc1" => self.names.long = rest.trim().to_string(),
            "toc2" => self.names.short = rest.trim().to_string(),
            "toc3" => self.names.abbrev = rest.trim().to_string(),
            "mt1" | "mt2" | "mt3" => self.names.title_lines.push(rest.trim().to_string()),
            "cl" => {
                let label = Some(rest.trim().to_string());
                match &mut self.chapter {
                    Some(chapter) => chapter.label = label,
                    None if self.chapters.is_empty() => self.chapter_label = label,
                    None => return Err(self.malformed("\\cl between chapters")),
                }
            }
            "cp" => match &mut self.chapter {
                Some(chapter) => chapter.published_number = Some(rest.trim().to_string()),
                None => return Err(self.malformed("\\cp outside a chapter")),
            },
            "c" => {
                let number: u32 = rest
                    .trim()
                    .parse()
                    .map_err(|_| self.malformed(format!("bad chapter number {:?}", rest.trim())))?;
                self.close_block();
                self.close_chapter();
                self.chapter = Some(Chapter {
                    number,
                    label: None,
                    published_number: None,
                    blocks: Vec::new(),
                });
            }
            name => {
                if let Some(kind) = block_kind(name) {
                    self.close_block();
                    self.block = Some(Block {
                        kind,
                        content: Vec::new(),
                    });
                    self.at_start = true;
                    self.inline(rest)?;
                } else if self.block.is_some() {
                    // A `\v …` line (or any inline marker) continuing the
                    // current block. Parse the whole line, marker included.
                    self.inline(line)?;
                } else {
                    return Err(self.malformed(format!("\\{name} with no open block")));
                }
            }
        }
        Ok(())
    }

    /// Finish the current block, if any, and file it under the current
    /// chapter or (before the first `\c`) the book introduction.
    fn close_block(&mut self) {
        let Some(mut block) = self.block.take() else {
            return;
        };
        // Trailing whitespace on the last run is line-end noise, not text.
        if let Some(Inline::Text(run)) = block.content.last_mut() {
            let trimmed = run.text.trim_end().len();
            run.text.truncate(trimmed);
        }
        block.content.retain(|inline| match inline {
            Inline::Text(run) => !run.text.is_empty(),
            _ => true,
        });
        match &mut self.chapter {
            Some(chapter) => chapter.blocks.push(block),
            None => self.intro.push(block),
        }
    }

    fn close_chapter(&mut self) {
        if let Some(chapter) = self.chapter.take() {
            self.chapters.push(chapter);
        }
    }

    fn finish(mut self) -> Result<Book, Error> {
        self.close_block();
        self.close_chapter();
        if let Some(style) = self.styles.last() {
            return Err(self.malformed(format!("style {style:?} still open at end of file")));
        }
        let code = self.code.take().ok_or_else(|| self.malformed("file has no \\id line"))?;
        let section = self.section.take().expect("section is set together with code");
        Ok(Book {
            file: self.file.to_string(),
            code,
            section,
            names: self.names,
            chapter_label: self.chapter_label,
            intro: self.intro,
            chapters: self.chapters,
        })
    }

    // ── inline content ─────────────────────────────────────────────────────

    fn style(&self) -> TextStyle {
        self.styles.last().copied().unwrap_or(TextStyle::Normal)
    }

    /// Open (`\wj `) or close (`\wj*`) a character style.
    fn set_style(&mut self, style: TextStyle, closing: bool) -> Result<(), Error> {
        if closing {
            match self.styles.pop() {
                Some(open) if open == style => Ok(()),
                Some(open) => Err(self.malformed(format!("closing {style:?} while {open:?} is open"))),
                None => Err(self.malformed(format!("closing {style:?} with nothing open"))),
            }
        } else {
            self.styles.push(style);
            Ok(())
        }
    }

    /// The text run at the end of the current block, if it has this style;
    /// otherwise a new empty run of this style is started.
    fn run_for(&mut self, style: TextStyle) -> &mut TextRun {
        let content = &mut self.block.as_mut().expect("run_for called with no open block").content;
        let reuse = matches!(content.last(), Some(Inline::Text(run)) if run.style == style);
        if !reuse {
            content.push(Inline::Text(TextRun {
                text: String::new(),
                style,
                words: Vec::new(),
            }));
        }
        match content.last_mut() {
            Some(Inline::Text(run)) => run,
            _ => unreachable!(),
        }
    }

    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let style = self.style();
        let mut at_start = self.at_start;
        let run = self.run_for(style);
        append_collapsed(&mut run.text, text, &mut at_start);
        self.at_start = at_start;
    }

    /// A `\w word|strong="H1234"\w*`: the word joins the current run, and its
    /// byte range is tagged.
    fn push_word(&mut self, word: &str, strong: StrongsId) {
        let style = self.style();
        let mut at_start = self.at_start;
        let run = self.run_for(style);
        let start = run.text.len();
        append_collapsed(&mut run.text, word, &mut at_start);
        let end = run.text.len();
        if end > start {
            run.words.push(WordTag {
                start: start as u32,
                end: end as u32,
                strong,
            });
        }
        self.at_start = at_start;
    }

    fn push_inline(&mut self, inline: Inline) {
        // After a verse number the next leading whitespace is syntax, not
        // text. After a footnote or cross-reference, spacing is real: "God¹
        // created" vs "God,¹ it".
        self.at_start = matches!(inline, Inline::Verse(_));
        self.block
            .as_mut()
            .expect("push_inline called with no open block")
            .content
            .push(inline);
    }

    /// Parse inline content (the rest of a block line, or a whole `\v` line)
    /// into the current block.
    fn inline(&mut self, text: &str) -> Result<(), Error> {
        let mut rest = text;
        while !rest.is_empty() {
            let Some(at) = rest.find('\\') else {
                self.push_text(rest);
                break;
            };
            self.push_text(&rest[..at]);
            let marker = read_marker(&rest[at..]).ok_or_else(|| self.malformed("stray backslash"))?;
            let after = &rest[at + marker.len..];

            rest = match (marker.name, marker.closing) {
                ("v", false) => {
                    let (token, after) = take_token(after);
                    let number = parse_verse_number(token)
                        .ok_or_else(|| self.malformed(format!("bad verse number {token:?}")))?;
                    self.push_inline(Inline::Verse(number));
                    after
                }
                ("w" | "+w", false) => {
                    let close = if marker.name == "w" { "\\w*" } else { "\\+w*" };
                    let end = after
                        .find(close)
                        .ok_or_else(|| self.malformed(format!("unclosed \\{}", marker.name)))?;
                    let (word, strong) = self.word_and_tag(&after[..end])?;
                    self.push_word(word, strong);
                    &after[end + close.len()..]
                }
                ("f", false) => {
                    let end = after.find("\\f*").ok_or_else(|| self.malformed("unclosed \\f"))?;
                    let note = self.footnote(&after[..end])?;
                    self.push_inline(Inline::Footnote(note));
                    &after[end + "\\f*".len()..]
                }
                ("x", false) => {
                    let end = after.find("\\x*").ok_or_else(|| self.malformed("unclosed \\x"))?;
                    let xref = self.cross_ref(&after[..end])?;
                    self.push_inline(Inline::CrossRef(xref));
                    &after[end + "\\x*".len()..]
                }
                ("wj", closing) => {
                    self.set_style(TextStyle::WordsOfJesus, closing)?;
                    after
                }
                ("qs", closing) => {
                    self.set_style(TextStyle::Selah, closing)?;
                    after
                }
                ("k", closing) => {
                    self.set_style(TextStyle::Keyword, closing)?;
                    after
                }
                ("bk" | "+bk", closing) => {
                    self.set_style(TextStyle::BookName, closing)?;
                    after
                }
                ("+wh", closing) => {
                    self.set_style(TextStyle::Hebrew, closing)?;
                    after
                }
                ("v" | "w" | "+w" | "f" | "x", true) => {
                    return Err(self.malformed(format!("\\{}* with nothing open", marker.name)));
                }
                (name, _) => return Err(self.unknown(name)),
            };
        }
        Ok(())
    }

    /// The inside of a `\w …\w*`: `heavens|strong="H8064"`.
    fn word_and_tag<'t>(&self, content: &'t str) -> Result<(&'t str, StrongsId), Error> {
        if content.contains('\\') {
            return Err(self.malformed(format!("marker inside a word tag: {content:?}")));
        }
        let (word, attrs) = content
            .split_once('|')
            .ok_or_else(|| self.malformed(format!("word tag without attributes: {content:?}")))?;
        let strong = parse_strong(attrs)
            .ok_or_else(|| self.malformed(format!("word tag with unexpected attributes: {attrs:?}")))?;
        Ok((word, strong))
    }

    /// The inside of a `\f …\f*`:
    /// `+ \fr 1:1 \ft The Hebrew word rendered “God” is “\+wh אֱלֹהִ֑ים\+wh*” (Elohim).`
    fn footnote(&self, inner: &str) -> Result<Footnote, Error> {
        // The caller ("+" = let the typesetter number it) is layout advice,
        // not text.
        let (_caller, mut rest) = take_token(inner);
        let mut reference = String::new();
        let mut content: Vec<TextRun> = Vec::new();
        // `\ft`, `\fq`, `\fqa`, `\fl` set the base style until the next one;
        // `\+wh` and `\+bk` are nested styles closed with `*`.
        let mut base = TextStyle::Normal;
        let mut nested: Vec<TextStyle> = Vec::new();
        let mut at_start = true;

        while !rest.is_empty() {
            let style = nested.last().copied().unwrap_or(base);
            let Some(at) = rest.find('\\') else {
                append_note_text(&mut content, style, rest, &mut at_start);
                break;
            };
            append_note_text(&mut content, style, &rest[..at], &mut at_start);
            let marker = read_marker(&rest[at..]).ok_or_else(|| self.malformed("stray backslash in footnote"))?;
            let after = &rest[at + marker.len..];

            rest = match (marker.name, marker.closing) {
                ("fr", false) => {
                    let end = after.find('\\').unwrap_or(after.len());
                    reference = after[..end].trim().to_string();
                    &after[end..]
                }
                ("ft", false) => {
                    base = TextStyle::Normal;
                    after
                }
                ("fq", false) => {
                    base = TextStyle::NoteQuote;
                    after
                }
                ("fqa", false) => {
                    base = TextStyle::NoteAlternate;
                    after
                }
                ("fl", false) => {
                    base = TextStyle::NoteLabel;
                    after
                }
                ("+wh", false) => {
                    nested.push(TextStyle::Hebrew);
                    after
                }
                ("+bk" | "bk", false) => {
                    nested.push(TextStyle::BookName);
                    after
                }
                ("+wh" | "+bk" | "bk", true) => {
                    nested
                        .pop()
                        .ok_or_else(|| self.malformed(format!("\\{}* in footnote with nothing open", marker.name)))?;
                    after
                }
                ("+w" | "w", false) => {
                    let close = if marker.name == "w" { "\\w*" } else { "\\+w*" };
                    let end = after
                        .find(close)
                        .ok_or_else(|| self.malformed(format!("unclosed \\{} in footnote", marker.name)))?;
                    let (word, strong) = self.word_and_tag(&after[..end])?;
                    append_note_word(&mut content, style, word, strong, &mut at_start);
                    &after[end + close.len()..]
                }
                (name, _) => return Err(self.unknown(name)),
            };
        }

        if !nested.is_empty() {
            return Err(self.malformed("footnote ends with a nested style still open"));
        }
        if let Some(last) = content.last_mut() {
            let trimmed = last.text.trim_end().len();
            last.text.truncate(trimmed);
        }
        content.retain(|run| !run.text.is_empty());
        Ok(Footnote { reference, content })
    }

    /// The inside of a `\x …\x*`: `+ \xo 1:23 \xt Isaiah 7:14`.
    fn cross_ref(&self, inner: &str) -> Result<CrossRef, Error> {
        let (_caller, mut rest) = take_token(inner);
        let mut origin = String::new();
        let mut targets: Vec<String> = Vec::new();

        while !rest.is_empty() {
            let at = rest
                .find('\\')
                .ok_or_else(|| self.malformed(format!("text outside a marker in cross-reference: {rest:?}")))?;
            if !rest[..at].trim().is_empty() {
                return Err(self.malformed(format!("text outside a marker in cross-reference: {:?}", &rest[..at])));
            }
            let marker = read_marker(&rest[at..]).ok_or_else(|| self.malformed("stray backslash in cross-reference"))?;
            let after = &rest[at + marker.len..];
            let end = after.find('\\').unwrap_or(after.len());
            let value = after[..end].trim();
            match (marker.name, marker.closing) {
                ("xo", false) => origin = value.to_string(),
                ("xt", false) => targets.push(value.to_string()),
                (name, _) => return Err(self.unknown(name)),
            }
            rest = &after[end..];
        }

        Ok(CrossRef {
            origin,
            targets: targets.join("; "),
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Small pure helpers
// ────────────────────────────────────────────────────────────────────────────

/// A marker as it appears in the text: `\wj ` → name "wj", not closing, and
/// `len` covers the backslash, the name and the one space that follows an
/// opening marker. `\wj*` → name "wj", closing.
struct Marker<'a> {
    name: &'a str,
    closing: bool,
    len: usize,
}

/// Read the marker at the start of `s` (which must begin with a backslash).
fn read_marker(s: &str) -> Option<Marker<'_>> {
    let body = s.strip_prefix('\\')?;
    let name_len = body
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'+')
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &body[..name_len];
    let tail = &body[name_len..];
    let closing = tail.starts_with('*');
    let mut len = 1 + name_len;
    if closing {
        len += 1;
    } else if tail.starts_with(' ') {
        len += 1;
    }
    Some(Marker { name, closing, len })
}

/// Split off the first whitespace-delimited token and the one space after it.
fn take_token(s: &str) -> (&str, &str) {
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    let token = &s[..end];
    let rest = s[end..].strip_prefix(' ').unwrap_or(&s[end..]);
    (token, rest)
}

/// `15` → 15..15, `15-16` → 15..16. Anything else is not a verse number.
fn parse_verse_number(token: &str) -> Option<VerseNumber> {
    match token.split_once('-') {
        Some((a, b)) => Some(VerseNumber {
            start: a.parse().ok()?,
            end: b.parse().ok()?,
        }),
        None => {
            let n = token.parse().ok()?;
            Some(VerseNumber { start: n, end: n })
        }
    }
}

/// `strong="H8064"` → Hebrew 8064. The publisher never uses any other
/// attribute, so anything else is an error at the call site.
fn parse_strong(attrs: &str) -> Option<StrongsId> {
    let value = attrs.trim().strip_prefix("strong=\"")?.strip_suffix('"')?;
    let (lang, digits) = value.split_at(1);
    let lang = match lang {
        "H" => StrongsLang::Hebrew,
        "G" => StrongsLang::Greek,
        _ => return None,
    };
    Some(StrongsId {
        lang,
        number: digits.parse().ok()?,
    })
}

/// Append `src` to `dst`, collapsing runs of whitespace to a single space.
/// While `at_start` is true, leading whitespace is dropped; it turns false at
/// the first real character.
fn append_collapsed(dst: &mut String, src: &str, at_start: &mut bool) {
    for ch in src.chars() {
        if ch.is_whitespace() {
            if *at_start || dst.ends_with(' ') {
                continue;
            }
            dst.push(' ');
        } else {
            *at_start = false;
            dst.push(ch);
        }
    }
}

/// Footnote flavour of `Parser::run_for`: the last run if it has this style,
/// else a new one.
fn note_run_for(runs: &mut Vec<TextRun>, style: TextStyle) -> &mut TextRun {
    let reuse = matches!(runs.last(), Some(run) if run.style == style);
    if !reuse {
        runs.push(TextRun {
            text: String::new(),
            style,
            words: Vec::new(),
        });
    }
    runs.last_mut().expect("just pushed")
}

fn append_note_text(runs: &mut Vec<TextRun>, style: TextStyle, text: &str, at_start: &mut bool) {
    if text.is_empty() {
        return;
    }
    let run = note_run_for(runs, style);
    append_collapsed(&mut run.text, text, at_start);
}

fn append_note_word(runs: &mut Vec<TextRun>, style: TextStyle, word: &str, strong: StrongsId, at_start: &mut bool) {
    let run = note_run_for(runs, style);
    let start = run.text.len();
    append_collapsed(&mut run.text, word, at_start);
    let end = run.text.len();
    if end > start {
        run.words.push(WordTag {
            start: start as u32,
            end: end as u32,
            strong,
        });
    }
}

/// Block-level markers, by name. Anything not here is either book metadata
/// (handled in `line_of`) or inline content.
fn block_kind(marker: &str) -> Option<BlockKind> {
    Some(match marker {
        "p" => BlockKind::Paragraph,
        "m" => BlockKind::Flush,
        "pi1" => BlockKind::Indented,
        "pc" => BlockKind::Centered,
        "mi" => BlockKind::IndentedFlush,
        "nb" => BlockKind::NoBreak,
        "li1" => BlockKind::ListItem,
        "q1" => BlockKind::Poetry(1),
        "q2" => BlockKind::Poetry(2),
        "q3" => BlockKind::Poetry(3),
        "b" => BlockKind::Blank,
        "d" => BlockKind::Superscription,
        "sp" => BlockKind::Speaker,
        "s1" => BlockKind::Heading,
        "ms1" => BlockKind::MajorHeading,
        "is1" => BlockKind::IntroHeading,
        "ip" => BlockKind::IntroParagraph,
        "ili" => BlockKind::IntroListItem,
        _ => return None,
    })
}

/// The 83 book codes the publisher ships, and the shelf each belongs on.
fn section_for(code: &str) -> Option<Section> {
    const OLD: [&str; 39] = [
        "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
        "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
        "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
    ];
    const DEUTERO: [&str; 15] = [
        "TOB", "JDT", "ESG", "WIS", "SIR", "BAR", "1MA", "2MA", "1ES", "MAN", "PS2", "3MA", "2ES",
        "4MA", "DAG",
    ];
    const NEW: [&str; 27] = [
        "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH",
        "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD",
        "REV",
    ];
    Some(match code {
        "FRT" => Section::FrontMatter,
        "GLO" => Section::Glossary,
        c if OLD.contains(&c) => Section::OldTestament,
        c if DEUTERO.contains(&c) => Section::Deuterocanon,
        c if NEW.contains(&c) => Section::NewTestament,
        _ => return None,
    })
}
