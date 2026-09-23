// ── Add your header block above ──

//! Sojourner: the window.
//!
//! Everything about the text lives in the library (`sojourner::text`,
//! `sojourner::crossrefs`); this file only decides what the reader is
//! looking at and draws it.
//!
//! The book opens on a title page. Turning the page goes through the
//! publisher's own preface, then the Old Testament, the New Testament, the
//! Deuterocanon, and finally the publisher's glossary — every file in the
//! zip, in Sojourner's shelf order. Pages turn with the arrows beside the
//! sheet or the arrow keys, and a Contents sidebar opens on the left to jump
//! anywhere.
//!
//! The page is a sheet of fixed size, whatever the window is: a chapter is
//! typeset into as many sheets as it needs, each holding what fits, and the
//! window shows one sheet at a time — centered on the desk when the window
//! is larger than the sheet, scrolling when it is smaller. Typesetting
//! measures the text with the same engine that draws it (see "Typesetting"
//! below), so what is measured to fit does fit, to the pixel. The one
//! setting so far, the text size, changes with Ctrl+plus and Ctrl+minus and
//! re-sets every page.
//!
//! The trail: verse numbers, footnote markers (†) and the translators' own
//! cross-reference markers (‡) are links. Clicking one opens the right-hand
//! panel on that verse — its text, its references with *their* text — and
//! clicking a reference in the panel shows *that* verse and its references,
//! growing a breadcrumb line the reader can walk back along. The page never
//! moves while peeking: it is the anchor. "Go there" turns the page to the
//! verse in the panel, making it the new anchor.
//!
//! Reading aloud: Play (or Space) reads the chapter on the page, verse by
//! verse, with the voice in `sojourner::voice`. The verse being spoken is
//! highlighted and the page turns to keep it in view; ⏮ and ⏭ (Shift+Up/
//! Down) go back a verse or skip one, Pause holds the place, and a chapter
//! that ends while its page is showing runs on into the next. "Read from
//! here" in the trail panel starts at any verse.
//!
//! The place the reader left off is remembered (through cosmic-config's
//! state store, as book code, chapter and verse, so it survives updates to
//! the text and changes of text size), and the book opens there next time.

use std::collections::HashMap;
use std::sync::Arc;

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use cosmic::iced::core::text::LineHeight;
use cosmic::iced::keyboard::{self, key::Named};
use cosmic::iced::futures::Stream;
use cosmic::iced::widget::scrollable::{scroll_by, scroll_to, AbsoluteOffset, Direction, Scrollbar};
use cosmic::iced::widget::text::{Rich, Span};
use cosmic::iced::widget::{mouse_area, rich_text, span, stack, Id};
use cosmic::iced::{self, window, Border, Color, Event, Font, Length, Pixels, Shadow, Size, Subscription, Vector};
use cosmic::widget::{button, column, container, divider, flex_row, icon, responsive, row, scrollable, space, text};
use cosmic::{Application, ApplicationExt, Element};
use serde::{Deserialize, Serialize};

// The library's module is also called `text`, which collides with the text
// *widget*; call the library side `scripture` in this file.
use sojourner::page::{
    chapter_title, geometry, is_air, paginate, set_content, shaped_span, typeset_chapter, Geometry, Ink, Leaf, PageLink, Par,
    Setter, Shape, DEFAULT_TEXT_SIZE, LARGEST_TEXT, MARGIN_X, MARGIN_Y, SERIF, SERIF_BOLD, SERIF_ITALIC, SHEET_HEIGHT,
    SHEET_WIDTH, SMALLEST_TEXT, TEXT_HEIGHT, TEXT_WIDTH,
};
use sojourner::text as scripture;
use sojourner::text::{CrossRef, Footnote, Inline, Section, TextStyle, VerseRef};
use sojourner::voice::reader::{self, Reader, Verse};
use sojourner::Library;

// ────────────────────────────────────────────────────────────────────────────
// Type
// ────────────────────────────────────────────────────────────────────────────

/// The sheet's size, margins and text sizes are the library's
/// (`sojourner::page`), since the typesetting that fills the sheets lives
/// there; this file only draws them.

/// Air around the sheet on the desk, and the width of the page-turn arrows
/// beside it.
const DESK_AIR: f32 = 16.0;
const ARROW_WIDTH: f32 = 56.0;

/// The panel sets text a little smaller than the page, and does not change.
const PANEL_BODY: f32 = 16.0;
const PANEL_LINE: LineHeight = LineHeight::Absolute(Pixels(PANEL_BODY * 1.5));

/// How much of a referenced verse the panel quotes under its link before
/// trailing off. Clicking the link shows the whole thing.
const QUOTE_CHARS: usize = 260;

/// How far one press of Up or Down scrolls a window too small for the
/// sheet: two lines of body text.
const SCROLL_STEP: f32 = DEFAULT_TEXT_SIZE * 1.5 * 2.0;

// ────────────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────────────

pub struct Sojourner {
    core: Core,
    /// The text, its index and the cross-reference web, once loaded. Shared
    /// so the load task can hand it over without copying it.
    library: Option<Arc<Library>>,
    /// Why the load failed, if it did.
    error: Option<String>,
    /// Everything that can be turned to, in reading order. See `shelf_order`.
    shelf: Vec<Slot>,
    /// Where the reader is: the anchor.
    at: Position,
    /// The body text size, in logical pixels.
    text_size: f32,
    /// Chapters typeset into sheets at the current text size, by (slot,
    /// chapter index). Filled as chapters are turned to; emptied when the
    /// text size changes.
    leaves: HashMap<(usize, usize), Vec<Leaf>>,
    /// Whether the Contents sidebar is open on the left.
    contents_open: bool,
    /// The window's size, as last reported, for deciding whether the
    /// Contents sidebar can sit beside the sheet or must lie over the desk.
    window: Size,
    /// The book whose chapter numbers are unfolded in the Contents sidebar.
    expanded: Option<usize>,
    /// Where the reader has peeked, in order. `trail[0]` is what they clicked
    /// on the page; the last stop is what the panel shows.
    trail: Vec<Stop>,
    /// The voice thread's handle, once its subscription has started it.
    reader: Option<Reader>,
    /// What is being read aloud, if anything.
    reading: Option<Reading>,
    /// A line about the voice for the person to see: that it is loading, or
    /// why it can't read.
    voice_notice: Option<String>,
}

/// Where the reader left off, as it is saved: the publisher's book code
/// ("JHN"), the chapter number and the first verse that begins on the
/// sheet (so the book reopens on that very sheet), or an empty code for
/// the title page. Codes and numbers rather than positions,
/// so the place survives a change of edition, shelf order or text size.
/// (Places saved before there were verses have none; that reads as 0, the
/// start of the chapter.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Place {
    book: String,
    chapter: u32,
    #[serde(default)]
    verse: u32,
}

/// A chapter being read aloud.
#[derive(Debug, Clone)]
struct Reading {
    book: u8,
    chapter: u32,
    /// The verses handed to the voice, in order.
    verses: Vec<VerseRef>,
    /// The one being spoken (an index into `verses`).
    index: usize,
    paused: bool,
}

/// One thing on the shelf: Sojourner's own title page, or one of the
/// publisher's files (a book of scripture, the preface, the glossary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Title,
    /// Index into `Bible::books`.
    Book(usize),
}

/// A place on the shelf: which slot, which chapter of it, and which sheet
/// of the chapter. For a book of scripture the page is the chapter
/// (0-based); the title page, preface and glossary each have a single page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    slot: usize,
    page: usize,
    leaf: usize,
}

/// One stop on the trail.
#[derive(Debug, Clone)]
enum Stop {
    /// A verse, or a range of verses, and its cross-references.
    Verses { start: VerseRef, end: VerseRef },
    /// A translator's footnote, lifted out of its verse.
    Note { at: VerseRef, note: Footnote },
    /// One of the translators' own cross-references, lifted out of its verse.
    Xref { at: VerseRef, xref: CrossRef },
}

#[derive(Debug, Clone)]
pub enum Message {
    /// The background load finished, with the library or an error.
    Loaded(Result<Arc<Library>, String>),
    NextPage,
    PreviousPage,
    /// Scroll a window too small for the sheet by this many pixels
    /// (negative is up).
    ScrollPage(f32),
    /// Make the text this much larger (negative: smaller); 0 resets it.
    TextSize(f32),
    /// Open or close the Contents sidebar.
    ToggleContents,
    /// The window was resized (or opened) to this size.
    WindowResized(Size),
    /// Escape, or the drawer's close button: close the trail if it is open,
    /// else the Contents sidebar.
    ClosePanel,
    /// Unfold (or fold) a book's chapter numbers in the Contents panel.
    ExpandBook(usize),
    /// Turn to a page.
    GoTo(Position),
    /// A link on the page (or in the panel) was clicked: start a trail.
    Follow(PageLink),
    /// A reference in the panel was clicked: add a stop to the trail.
    Peek(VerseRef, VerseRef),
    /// A breadcrumb was clicked: cut the trail back to that stop.
    Backtrack(usize),
    /// "Go there": turn the page to a verse, making it the anchor.
    GoToVerse(VerseRef),
    /// The voice thread is up; here is its handle.
    ReaderStarted(Reader),
    /// Word from the voice thread.
    Reader(reader::Event),
    /// Play, or pause, or resume: what the one button means depends on the
    /// state.
    PlayPause,
    /// Go there and start reading aloud at this verse.
    ReadFrom(VerseRef),
    PreviousVerse,
    NextVerse,
    StopReading,
}

/// The scrollable a small window shows the sheet through, so a page turn
/// can put it back to the top.
fn page_scroll_id() -> Id {
    Id::new("sojourner-page")
}

impl Sojourner {
    /// How many chapters (pages) a slot has.
    fn pages(&self, slot: usize) -> usize {
        match (self.shelf[slot], &self.library) {
            (Slot::Title, _) => 1,
            (Slot::Book(i), Some(lib)) => lib.bible.books[i].chapters.len().max(1),
            (Slot::Book(_), None) => 1,
        }
    }

    /// The sheets of a chapter, typeset if they haven't been yet.
    fn leaves_of(&mut self, slot: usize, page: usize) -> &[Leaf] {
        let Slot::Book(i) = self.shelf[slot] else { return &[] };
        let Some(lib) = self.library.clone() else { return &[] };
        self.leaves.entry((slot, page)).or_insert_with(|| {
            let pars = typeset_chapter(&lib.bible.books[i], i as u8, page);
            paginate(pars, self.text_size)
        })
    }

    /// How many sheets a chapter has (one for the title page).
    fn leaf_count(&mut self, slot: usize, page: usize) -> usize {
        match self.shelf[slot] {
            Slot::Title => 1,
            Slot::Book(_) => self.leaves_of(slot, page).len().max(1),
        }
    }

    /// The sheet a verse begins on: the one carrying its number, or failing
    /// that the one whose verses surround it (a bridged verse asked for by
    /// its second number), or the first.
    fn leaf_of_verse(&mut self, slot: usize, page: usize, verse: u32) -> usize {
        let leaves = self.leaves_of(slot, page);
        let numbered = leaves.iter().position(|leaf| {
            leaf.pars
                .iter()
                .flat_map(|par| par.pieces.iter())
                .any(|piece| matches!(piece.link, Some(PageLink::Verse(v)) if v.verse == verse))
        });
        numbered
            .or_else(|| {
                leaves
                    .iter()
                    .position(|leaf| leaf.first_verse.is_some_and(|f| f <= verse) && leaf.last_verse.is_some_and(|l| verse <= l))
            })
            .unwrap_or(0)
    }

    /// The sheet being shown, if it is one of a chapter.
    fn leaf(&self) -> Option<&Leaf> {
        self.leaves.get(&(self.at.slot, self.at.page))?.get(self.at.leaf)
    }

    /// Turn one sheet forward or back, crossing from chapter to chapter and
    /// slot to slot along the shelf, and stopping at either cover.
    fn turn(&mut self, forward: bool) {
        if self.library.is_none() {
            return;
        }
        let at = self.at;
        self.at = if forward {
            if at.leaf + 1 < self.leaf_count(at.slot, at.page) {
                Position { leaf: at.leaf + 1, ..at }
            } else if at.page + 1 < self.pages(at.slot) {
                Position { slot: at.slot, page: at.page + 1, leaf: 0 }
            } else if at.slot + 1 < self.shelf.len() {
                Position { slot: at.slot + 1, page: 0, leaf: 0 }
            } else {
                at
            }
        } else if at.leaf > 0 {
            Position { leaf: at.leaf - 1, ..at }
        } else if at.page > 0 {
            let page = at.page - 1;
            Position { slot: at.slot, page, leaf: self.leaf_count(at.slot, page) - 1 }
        } else if at.slot > 0 {
            let slot = at.slot - 1;
            let page = self.pages(slot) - 1;
            Position { slot, page, leaf: self.leaf_count(slot, page) - 1 }
        } else {
            at
        };
    }

    /// The sheet a verse is on.
    fn position_of(&mut self, verse: VerseRef) -> Option<Position> {
        let lib = self.library.clone()?;
        let slot = self.shelf.iter().position(|s| *s == Slot::Book(usize::from(verse.book)))?;
        let book = &lib.bible.books[usize::from(verse.book)];
        let page = book.chapters.iter().position(|c| c.number == verse.chapter)?;
        let leaf = self.leaf_of_verse(slot, page, verse.verse);
        Some(Position { slot, page, leaf })
    }

    /// A page turn or a jump: a window too small for the sheet scrolls back
    /// to its top. (A window that shows the whole sheet has nothing to
    /// scroll, and the task finds nothing to do.)
    fn back_to_top() -> Task<Message> {
        scroll_to(page_scroll_id(), AbsoluteOffset { x: 0.0, y: 0.0 }.into())
    }

    /// What a click on the page opens the panel on.
    fn stop_for(&self, link: PageLink) -> Option<Stop> {
        let lib = self.library.as_ref()?;
        match link {
            PageLink::Verse(v) => Some(Stop::Verses { start: v, end: v }),
            PageLink::Note(at, nth) => {
                let found = scripture::verse(&lib.bible, at)?;
                let note = found
                    .pieces
                    .iter()
                    .flat_map(|(_, inlines)| inlines.iter())
                    .filter_map(|inline| match inline {
                        Inline::Footnote(note) => Some(note),
                        _ => None,
                    })
                    .nth(nth)?
                    .clone();
                Some(Stop::Note { at, note })
            }
            PageLink::Xref(at, nth) => {
                let found = scripture::verse(&lib.bible, at)?;
                let xref = found
                    .pieces
                    .iter()
                    .flat_map(|(_, inlines)| inlines.iter())
                    .filter_map(|inline| match inline {
                        Inline::CrossRef(xref) => Some(xref),
                        _ => None,
                    })
                    .nth(nth)?
                    .clone();
                Some(Stop::Xref { at, xref })
            }
        }
    }

    /// Hand the voice a chapter from `from` onward and remember what was
    /// handed over, so the page can follow. Only the words of each verse go
    /// to the voice: no numbers, no note markers, and nothing outside the
    /// verse flow (headings, superscriptions).
    fn start_reading(&mut self, from: VerseRef) {
        let Some(lib) = &self.library else { return };
        let Some(reader) = &self.reader else {
            self.voice_notice = Some("The voice isn't ready yet.".to_string());
            return;
        };
        let book = &lib.bible.books[usize::from(from.book)];
        let verses: Vec<Verse> = scripture::plain_verses(book)
            .into_iter()
            .filter(|(chapter, _, _)| *chapter == from.chapter)
            .map(|(chapter, number, text)| Verse {
                at: VerseRef { book: from.book, chapter, verse: number.start },
                text,
            })
            .collect();
        if verses.is_empty() {
            return;
        }
        // A bridged verse (15-16) asked for as 16 starts at its marker, 15.
        let index = verses
            .iter()
            .rposition(|v| v.at.verse <= from.verse)
            .unwrap_or(0);
        self.reading = Some(Reading {
            book: from.book,
            chapter: from.chapter,
            verses: verses.iter().map(|v| v.at).collect(),
            index,
            paused: false,
        });
        reader.read(verses, index);
    }

    /// The verse being read aloud, if the chapter on the page is the one
    /// being read.
    fn spoken_on_page(&self) -> Option<u32> {
        let reading = self.reading.as_ref()?;
        let Slot::Book(i) = self.shelf[self.at.slot] else { return None };
        let lib = self.library.as_ref()?;
        let book = &lib.bible.books[i];
        let chapter = book.chapters.get(self.at.page)?;
        (i as u8 == reading.book && chapter.number == reading.chapter).then(|| reading.verses[reading.index].verse)
    }

    /// Keep the spoken verse in view: if the chapter on the page is the one
    /// being read and the verse begins on another of its sheets, turn to
    /// that sheet.
    fn follow_along(&mut self) -> Task<Message> {
        let Some(spoken) = self.spoken_on_page() else { return Task::none() };
        let leaf = self.leaf_of_verse(self.at.slot, self.at.page, spoken);
        if leaf == self.at.leaf {
            return Task::none();
        }
        self.at.leaf = leaf;
        Self::back_to_top()
    }

    /// The chapter finished. If its page is still showing, the listener is
    /// following along: turn to the next chapter and keep reading. Otherwise
    /// stop where the text stops.
    fn continue_reading(&mut self) -> Task<Message> {
        let following = self.spoken_on_page().is_some();
        self.reading = None;
        if !following {
            return Task::none();
        }
        let before = self.at;
        self.turn(true);
        if self.at == before {
            return Task::none();
        }
        let Some(lib) = &self.library else { return Task::none() };
        let Slot::Book(i) = self.shelf[self.at.slot] else { return Task::none() };
        let Some(chapter) = lib.bible.books[i].chapters.get(self.at.page) else {
            // The next page has no chapters (the glossary): the reading ends.
            return Self::back_to_top();
        };
        let first = VerseRef { book: i as u8, chapter: chapter.number, verse: 1 };
        self.start_reading(first);
        Self::back_to_top()
    }

    /// The state store the place is kept in: cosmic-config's state directory,
    /// under the app id. None if there is no such directory to be had.
    fn place_store() -> Option<Config> {
        Config::new_state(Self::APP_ID, 1).ok()
    }

    /// The settings store (cosmic-config's config directory): the text size.
    fn settings_store() -> Option<Config> {
        Config::new(Self::APP_ID, 1).ok()
    }

    /// The place the page shows, in saved form.
    fn place(&self) -> Option<Place> {
        let lib = self.library.as_ref()?;
        Some(match self.shelf[self.at.slot] {
            Slot::Title => Place { book: String::new(), chapter: 0, verse: 0 },
            Slot::Book(i) => Place {
                book: lib.bible.books[i].code.clone(),
                chapter: lib.bible.books[i].chapters.get(self.at.page).map_or(0, |c| c.number),
                verse: self.leaf().and_then(|leaf| leaf.first_numbered().or(leaf.first_verse)).unwrap_or(0),
            },
        })
    }

    /// Save the place the page shows. Failures are not worth stopping for:
    /// the book still opens, just at the title page next time.
    fn remember_place(&self) {
        if let (Some(store), Some(place)) = (Self::place_store(), self.place()) {
            if let Err(e) = store.set("place", place) {
                eprintln!("couldn't save the place: {e}");
            }
        }
    }

    /// The saved place as a position on the shelf, if it still exists.
    fn saved_position(&mut self) -> Option<Position> {
        let place: Place = Self::place_store()?.get("place").ok()?;
        let lib = self.library.clone()?;
        if place.book.is_empty() {
            return Some(Position { slot: 0, page: 0, leaf: 0 });
        }
        let (i, book) = lib.bible.books.iter().enumerate().find(|(_, b)| b.code == place.book)?;
        let slot = self.shelf.iter().position(|s| *s == Slot::Book(i))?;
        let page = if book.chapters.is_empty() {
            0
        } else {
            book.chapters.iter().position(|c| c.number == place.chapter)?
        };
        let leaf = if place.verse == 0 { 0 } else { self.leaf_of_verse(slot, page, place.verse) };
        Some(Position { slot, page, leaf })
    }

    /// What the header shows: the chapter on the page, or the app's name on
    /// the title page.
    fn heading(&self) -> String {
        let Some(lib) = &self.library else { return "Sojourner".to_string() };
        match self.shelf.get(self.at.slot) {
            Some(Slot::Book(i)) => chapter_title(&lib.bible.books[*i], self.at.page),
            _ => "Sojourner".to_string(),
        }
    }

    /// The breadcrumb text for a stop.
    fn stop_label(&self, stop: &Stop) -> String {
        let Some(lib) = &self.library else {
            return String::new();
        };
        match stop {
            Stop::Verses { start, end } => scripture::reference_label(&lib.bible, *start, *end),
            Stop::Note { at, .. } => format!("Note · {}", scripture::reference_label(&lib.bible, *at, *at)),
            Stop::Xref { at, .. } => format!("‡ {}", scripture::reference_label(&lib.bible, *at, *at)),
        }
    }
}

/// Sojourner's shelf order: the title page, the publisher's preface, the Old
/// Testament, the New Testament, the Deuterocanon, the glossary. The
/// publisher's file order (which `Bible::books` keeps) puts the Deuterocanon
/// between the testaments; the reorder happens here, at view time, and
/// nowhere else. See assets/SOURCES.md.
fn shelf_order(lib: &Library) -> Vec<Slot> {
    let mut shelf = vec![Slot::Title];
    for section in [
        Section::FrontMatter,
        Section::OldTestament,
        Section::NewTestament,
        Section::Deuterocanon,
        Section::Glossary,
    ] {
        shelf.extend(
            lib.bible
                .books
                .iter()
                .enumerate()
                .filter(|(_, book)| book.section == section)
                .map(|(i, _)| Slot::Book(i)),
        );
    }
    shelf
}

/// The heading for a section of the shelf, as the Contents panel shows it.
fn section_name(section: Section) -> &'static str {
    match section {
        Section::FrontMatter => "Front matter",
        Section::OldTestament => "Old Testament",
        Section::NewTestament => "New Testament",
        Section::Deuterocanon => "Deuterocanon",
        Section::Glossary => "Back matter",
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Application
// ────────────────────────────────────────────────────────────────────────────

impl Application for Sojourner {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.hmrdsmoke.Sojourner";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.header_title = "Sojourner".to_string();
        let text_size = Self::settings_store()
            .and_then(|store| store.get::<f32>("text_size").ok())
            .map_or(DEFAULT_TEXT_SIZE, |size| size.clamp(SMALLEST_TEXT, LARGEST_TEXT));
        let mut app = Sojourner {
            core,
            library: None,
            error: None,
            shelf: Vec::new(),
            at: Position { slot: 0, page: 0, leaf: 0 },
            text_size,
            leaves: HashMap::new(),
            contents_open: false,
            window: window_size(),
            expanded: None,
            trail: Vec::new(),
            reader: None,
            reading: None,
            voice_notice: None,
        };
        let title = app.set_window_title("Sojourner".to_string());
        // Parse the bundled text and cross-references off the UI thread.
        // Well under a second in a release build, a few seconds in debug;
        // the window opens and says so meanwhile.
        let load = cosmic::task::future(async { Message::Loaded(Library::load_bundled().map(Arc::new)) });
        (app, Task::batch([title, load]))
    }

    /// Every message goes through `handle`; if it moved the page, the new
    /// place is saved, and the header and window title follow the chapter.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        let before = self.at;
        let task = self.handle(message);
        if self.at != before {
            self.remember_place();
        }
        let heading = self.heading();
        if heading != self.core.window.header_title {
            self.core.window.header_title = heading.clone();
            let title = self.set_window_title(if heading == "Sojourner" {
                heading
            } else {
                format!("{heading} – Sojourner")
            });
            return Task::batch([task, title]);
        }
        task
    }

    /// Left/Right (and PageUp/PageDown) turn pages; Up/Down scroll a window
    /// too small for the sheet; Space plays or pauses; Shift+Up/Down go back
    /// a verse or skip one; Ctrl+plus/minus/0 change the text size. Only
    /// key presses no widget claimed are considered, so typing in a search
    /// box later won't turn pages. The second subscription is the voice
    /// thread, which lives as long as the window.
    fn subscription(&self) -> Subscription<Self::Message> {
        let keys = iced::event::listen_with(|event, status, _window| {
            if status != iced::event::Status::Ignored {
                return None;
            }
            let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
                return None;
            };
            match key {
                // The space bar arrives as the character it types.
                keyboard::Key::Character(c) if c == " " => Some(Message::PlayPause),
                keyboard::Key::Character(c) if modifiers.control() => match c.as_str() {
                    "+" | "=" => Some(Message::TextSize(1.0)),
                    "-" | "_" => Some(Message::TextSize(-1.0)),
                    "0" => Some(Message::TextSize(0.0)),
                    _ => None,
                },
                keyboard::Key::Named(key) => match key {
                    Named::ArrowRight | Named::PageDown => Some(Message::NextPage),
                    Named::ArrowLeft | Named::PageUp => Some(Message::PreviousPage),
                    Named::ArrowDown if modifiers.shift() => Some(Message::NextVerse),
                    Named::ArrowUp if modifiers.shift() => Some(Message::PreviousVerse),
                    Named::ArrowDown => Some(Message::ScrollPage(SCROLL_STEP)),
                    Named::ArrowUp => Some(Message::ScrollPage(-SCROLL_STEP)),
                    Named::Escape => Some(Message::ClosePanel),
                    _ => None,
                },
                _ => None,
            }
        });
        let resizes = window::resize_events().map(|(_, size)| Message::WindowResized(size));
        Subscription::batch([keys, resizes, Subscription::run(voice_thread)])
    }

    /// The header stays quiet: the Contents toggle at the left, the reading
    /// controls at the right, the chapter's name between them. Pages turn
    /// from the arrows beside the sheet and the keys.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        if self.library.is_none() {
            return Vec::new();
        }
        vec![button::icon(icon::from_name("open-menu-symbolic")).on_press(Message::ToggleContents).into()]
    }

    /// The reading controls. Play shows on any chapter page; the rest only
    /// while something is being read.
    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let Some(lib) = &self.library else {
            return Vec::new();
        };
        let on_chapter = match self.shelf[self.at.slot] {
            Slot::Book(i) => !lib.bible.books[i].chapters.is_empty(),
            Slot::Title => false,
        };
        let mut controls = row![].spacing(2);
        if let Some(reading) = &self.reading {
            let play_icon = if reading.paused { "media-playback-start-symbolic" } else { "media-playback-pause-symbolic" };
            controls = controls
                .push(button::icon(icon::from_name("media-skip-backward-symbolic")).on_press(Message::PreviousVerse))
                .push(button::icon(icon::from_name(play_icon)).on_press(Message::PlayPause))
                .push(button::icon(icon::from_name("media-skip-forward-symbolic")).on_press(Message::NextVerse))
                .push(button::icon(icon::from_name("media-playback-stop-symbolic")).on_press(Message::StopReading));
        } else if on_chapter {
            controls = controls.push(button::icon(icon::from_name("media-playback-start-symbolic")).on_press(Message::PlayPause));
        }
        vec![controls.into()]
    }

    /// The right-hand drawer carries the trail, and nothing else.
    fn context_drawer(&self) -> Option<ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }
        let title = self.trail.last().map(|s| self.stop_label(s)).unwrap_or_default();
        Some(context_drawer::context_drawer(self.trail_panel(), Message::ClosePanel).title(title))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if self.library.is_none() {
            let notice = match &self.error {
                Some(error) => format!("Couldn't open the text: {error}"),
                None => "Opening the World English Bible…".to_string(),
            };
            return container(text(notice).size(18))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let palette = Palette::current();
        let mut page_area = column![].width(Length::Fill).height(Length::Fill);
        if let Some(notice) = &self.voice_notice {
            // A word about the voice, above the desk, only while there is
            // something to say.
            page_area = page_area.push(
                container(text(notice.as_str()).size(13).class(palette.muted))
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .padding([6, 0, 0, 0]),
            );
        }
        // The desk lays the sheet out for the room it has: centered when the
        // window shows it whole, scrolling when it doesn't. Its colour is
        // Sojourner's own, like the paper's, so the book looks the same on
        // every desktop theme.
        let page_area = page_area.push(responsive(move |size| self.desk(size)));
        let desk = container(page_area).width(Length::Fill).height(Length::Fill).class(desk_style(&palette));

        if !self.contents_open {
            return desk.into();
        }
        if self.contents_beside() {
            // Room for both: the sidebar sits beside the sheet.
            row![self.contents(), divider::vertical::light(), desk]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            // Not enough room: the sidebar lies over the desk like a drawer,
            // the sheet stays where it is, and a click on the desk beside it
            // (or a chapter chosen from it, or Escape) closes it.
            let drawer = container(self.contents()).height(Length::Fill).class(drawer_style());
            let beside = mouse_area(container(space::horizontal()).width(Length::Fill).height(Length::Fill))
                .on_press(Message::ClosePanel);
            stack![desk, row![drawer, beside].width(Length::Fill).height(Length::Fill)]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}

impl Sojourner {
    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(library)) => {
                self.shelf = shelf_order(&library);
                self.library = Some(library);
                // Open where the reader left off, if that place still exists.
                self.at = self.saved_position().unwrap_or(Position { slot: 0, page: 0, leaf: 0 });
                if self.at.slot != 0 {
                    self.expanded = Some(self.at.slot);
                }
                self.leaves_of(self.at.slot, self.at.page);
                Task::none()
            }
            Message::Loaded(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::NextPage => {
                self.turn(true);
                self.leaves_of(self.at.slot, self.at.page);
                Self::back_to_top()
            }
            Message::PreviousPage => {
                self.turn(false);
                self.leaves_of(self.at.slot, self.at.page);
                Self::back_to_top()
            }
            Message::ScrollPage(by) => scroll_by(page_scroll_id(), AbsoluteOffset { x: 0.0, y: by }),
            Message::TextSize(delta) => {
                let size = if delta == 0.0 {
                    DEFAULT_TEXT_SIZE
                } else {
                    (self.text_size + delta).clamp(SMALLEST_TEXT, LARGEST_TEXT)
                };
                if size == self.text_size {
                    return Task::none();
                }
                // Every sheet is set afresh at the new size; the page stays
                // on the verse it was showing.
                let verse = self.leaf().and_then(|leaf| leaf.first_numbered().or(leaf.first_verse));
                self.text_size = size;
                self.leaves.clear();
                self.at.leaf = match verse {
                    Some(v) => self.leaf_of_verse(self.at.slot, self.at.page, v),
                    None => 0,
                };
                if let Some(store) = Self::settings_store() {
                    if let Err(e) = store.set("text_size", size) {
                        eprintln!("couldn't save the text size: {e}");
                    }
                }
                Self::back_to_top()
            }
            Message::WindowResized(size) => {
                self.window = size;
                Task::none()
            }
            Message::ToggleContents => {
                self.contents_open = !self.contents_open;
                if self.contents_open {
                    // Unfold the book being read, so its chapter bookmark is
                    // in view straight away.
                    self.expanded = Some(self.at.slot);
                }
                Task::none()
            }
            Message::ClosePanel => {
                if self.core.window.show_context {
                    self.core.window.show_context = false;
                } else {
                    self.contents_open = false;
                }
                Task::none()
            }
            Message::ExpandBook(slot) => {
                self.expanded = if self.expanded == Some(slot) { None } else { Some(slot) };
                Task::none()
            }
            Message::GoTo(position) => {
                // The sidebar stays open when it sits beside the sheet:
                // turning through chapters from it is what it is for. Lying
                // over the sheet, it closes to show what was chosen.
                if !self.contents_beside() {
                    self.contents_open = false;
                }
                self.at = position;
                self.leaves_of(self.at.slot, self.at.page);
                Self::back_to_top()
            }
            Message::Follow(link) => {
                if let Some(stop) = self.stop_for(link) {
                    self.trail = vec![stop];
                    self.core.window.show_context = true;
                }
                Task::none()
            }
            Message::Peek(start, end) => {
                self.trail.push(Stop::Verses { start, end });
                self.core.window.show_context = true;
                Task::none()
            }
            Message::Backtrack(index) => {
                self.trail.truncate(index + 1);
                Task::none()
            }
            Message::GoToVerse(verse) => {
                if let Some(position) = self.position_of(verse) {
                    self.at = position;
                    self.expanded = Some(position.slot);
                }
                // A new anchor: the old trail belonged to the old one.
                self.trail.clear();
                self.core.window.show_context = false;
                Self::back_to_top()
            }

            // Reading aloud.
            Message::ReaderStarted(reader) => {
                self.reader = Some(reader);
                Task::none()
            }
            Message::PlayPause => {
                match (&self.reading, &self.reader) {
                    (Some(reading), Some(reader)) => {
                        reader.send(if reading.paused { reader::Command::Resume } else { reader::Command::Pause });
                        Task::none()
                    }
                    _ => {
                        // Nothing is being read: read the chapter on the page
                        // from the first verse that begins on this sheet.
                        let Slot::Book(i) = self.shelf[self.at.slot] else { return Task::none() };
                        let Some(lib) = &self.library else { return Task::none() };
                        let Some(chapter) = lib.bible.books[i].chapters.get(self.at.page) else { return Task::none() };
                        let verse = self.leaf().and_then(Leaf::first_numbered).unwrap_or(1).max(1);
                        let from = VerseRef { book: i as u8, chapter: chapter.number, verse };
                        self.start_reading(from);
                        Task::none()
                    }
                }
            }
            Message::ReadFrom(verse) => {
                // Like "Go there", then read from that verse.
                if let Some(position) = self.position_of(verse) {
                    self.at = position;
                    self.expanded = Some(position.slot);
                }
                self.trail.clear();
                self.core.window.show_context = false;
                self.start_reading(verse);
                if self.reading.is_some() {
                    self.follow_along()
                } else {
                    Self::back_to_top()
                }
            }
            Message::PreviousVerse => {
                if let (Some(_), Some(reader)) = (&self.reading, &self.reader) {
                    reader.send(reader::Command::Previous);
                }
                Task::none()
            }
            Message::NextVerse => {
                if let (Some(_), Some(reader)) = (&self.reading, &self.reader) {
                    reader.send(reader::Command::Next);
                }
                Task::none()
            }
            Message::StopReading => {
                if let Some(reader) = &self.reader {
                    reader.send(reader::Command::Stop);
                }
                self.reading = None;
                Task::none()
            }
            Message::Reader(event) => match event {
                reader::Event::Loading => {
                    self.voice_notice = Some("Loading the voice…".to_string());
                    Task::none()
                }
                reader::Event::Ready { .. } => {
                    self.voice_notice = None;
                    Task::none()
                }
                reader::Event::Unavailable(why) => {
                    self.voice_notice = Some(why);
                    self.reading = None;
                    Task::none()
                }
                reader::Event::Speaking { index, .. } => {
                    if let Some(reading) = &mut self.reading {
                        if index < reading.verses.len() {
                            reading.index = index;
                        }
                        reading.paused = false;
                    }
                    self.follow_along()
                }
                reader::Event::Paused => {
                    if let Some(reading) = &mut self.reading {
                        reading.paused = true;
                    }
                    Task::none()
                }
                reader::Event::Resumed => {
                    if let Some(reading) = &mut self.reading {
                        reading.paused = false;
                    }
                    Task::none()
                }
                reader::Event::Finished => self.continue_reading(),
                reader::Event::Stopped => Task::none(),
            },
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The desk and the sheet
// ────────────────────────────────────────────────────────────────────────────

impl Sojourner {
    /// The sheet on the desk with the page-turn arrows beside it. Given the
    /// room the window has: when the whole spread fits, it is centered;
    /// when it doesn't, it scrolls — down when only the height is short,
    /// both ways when the width is too.
    fn desk(&self, room: Size) -> Element<'_, Message> {
        let at_front = self.at == Position { slot: 0, page: 0, leaf: 0 };
        let at_back = {
            let last = self.shelf.len().saturating_sub(1);
            self.at.slot == last
                && self.at.page + 1 >= self.pages(last)
                && self.leaves.get(&(self.at.slot, self.at.page)).is_none_or(|l| self.at.leaf + 1 >= l.len())
        };
        let previous = button::icon(icon::from_name("go-previous-symbolic"))
            .medium()
            .on_press_maybe((!at_front).then_some(Message::PreviousPage));
        let next = button::icon(icon::from_name("go-next-symbolic"))
            .medium()
            .on_press_maybe((!at_back).then_some(Message::NextPage));
        // The arrows go when there is no room beside the sheet for them
        // (the Contents sidebar open in a narrow window); the keys still
        // turn the pages.
        let spread_width = SHEET_WIDTH + 2.0 * (ARROW_WIDTH + DESK_AIR);
        let with_arrows = room.width >= spread_width + DESK_AIR;
        let spread = if with_arrows {
            row![
                container(previous).width(ARROW_WIDTH).center_x(ARROW_WIDTH),
                self.sheet(),
                container(next).width(ARROW_WIDTH).center_x(ARROW_WIDTH),
            ]
            .spacing(DESK_AIR)
            .align_y(iced::Alignment::Center)
        } else {
            row![self.sheet()]
        };

        let fits_across = room.width >= (if with_arrows { spread_width } else { SHEET_WIDTH }) + DESK_AIR;
        let fits_down = room.height >= SHEET_HEIGHT + DESK_AIR;
        if fits_across && fits_down {
            return container(spread)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }
        let bar = || Scrollbar::new().width(8.0).scroller_width(8.0);
        let padded = container(spread).padding(DESK_AIR);
        if fits_across {
            scrollable(container(padded).width(Length::Fill).center_x(Length::Fill))
                .id(page_scroll_id())
                .direction(Direction::Vertical(bar()))
                .into()
        } else {
            scrollable(padded)
                .id(page_scroll_id())
                .direction(Direction::Both { vertical: bar(), horizontal: bar() })
                .into()
        }
    }

    /// One sheet of paper: the running head in the top margin, the text
    /// area, the folio in the bottom margin. On the title page, just the
    /// title.
    fn sheet(&self) -> Element<'_, Message> {
        let palette = Palette::current();
        let paper = paper_style(&palette);
        let Some(lib) = &self.library else { return space::horizontal().into() };

        let Slot::Book(i) = self.shelf[self.at.slot] else {
            return container(title_page(&palette))
                .width(SHEET_WIDTH)
                .height(SHEET_HEIGHT)
                .padding([MARGIN_Y, MARGIN_X])
                .class(paper)
                .into();
        };
        let book = &lib.bible.books[i];
        let leaves = self.leaves.get(&(self.at.slot, self.at.page));
        let leaf = leaves.and_then(|l| l.get(self.at.leaf));

        // The running head: the chapter at the left, the verses on this
        // sheet at the right, the way a printed Bible does it.
        let head_left = chapter_title(book, self.at.page);
        let head_right = match leaf.and_then(|l| l.first_verse.zip(l.last_verse)) {
            Some((first, last)) if first == last => first.to_string(),
            Some((first, last)) => format!("{first}–{last}"),
            None => String::new(),
        };
        let head = row![
            text(head_left).size(12).font(SERIF_ITALIC).class(palette.page_muted),
            space::horizontal(),
            text(head_right).size(12).font(SERIF_ITALIC).class(palette.page_muted),
        ]
        .height(MARGIN_Y)
        .align_y(iced::Alignment::Center);

        // The text area: the sheet's paragraphs, the first with no air above
        // it so every page starts at the same line.
        let spoken = self.spoken_on_page();
        let mut body = column![].spacing(0).width(TEXT_WIDTH);
        if let Some(leaf) = leaf {
            for (n, par) in leaf.pars.iter().enumerate() {
                body = body.push(par_element(par, self.text_size, &palette, spoken, n == 0));
            }
        }
        let area = container(body).width(TEXT_WIDTH).height(TEXT_HEIGHT).clip(true);

        // The folio: which sheet of the chapter this is.
        let folio = match leaves.map(Vec::len) {
            Some(count) if count > 1 => format!("{} of {count}", self.at.leaf + 1),
            _ => String::new(),
        };
        let foot = container(text(folio).size(12).font(SERIF).class(palette.page_muted))
            .width(Length::Fill)
            .height(MARGIN_Y)
            .center_x(Length::Fill)
            .center_y(MARGIN_Y);

        container(column![head, area, foot].width(TEXT_WIDTH))
            .width(SHEET_WIDTH)
            .height(SHEET_HEIGHT)
            .padding([0.0, MARGIN_X])
            .class(paper)
            .into()
    }
}

impl Sojourner {
    /// Whether the window is wide enough for the Contents sidebar to sit
    /// beside the whole sheet (the arrows may go; the sheet may not).
    fn contents_beside(&self) -> bool {
        self.window.width >= CONTENTS_WIDTH + 1.0 + SHEET_WIDTH + 3.0 * DESK_AIR
    }
}

/// The Contents sidebar as a drawer over the desk: the window's own
/// background, with a shadow along its edge.
fn drawer_style() -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(|theme| {
        let mut style = cosmic::theme::Container::background(theme.cosmic(), false);
        style.shadow = Shadow { color: Color { a: 0.3, ..Color::BLACK }, offset: Vector::new(2.0, 0.0), blur_radius: 12.0 };
        style
    })
}

/// The window to open with: one sheet whole, the arrows beside it and a
/// little desk around, under the header bar.
fn window_size() -> Size {
    let width = SHEET_WIDTH + 2.0 * (ARROW_WIDTH + DESK_AIR) + 3.0 * DESK_AIR;
    let height = SHEET_HEIGHT + 3.0 * DESK_AIR + 48.0;
    Size::new(width, height)
}

/// The desk: a plain surface under the sheet, in Sojourner's own colour.
fn desk_style(palette: &Palette) -> cosmic::theme::Container<'static> {
    let desk = palette.desk;
    cosmic::theme::Container::custom(move |_theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(desk)),
        ..Default::default()
    })
}

/// The look of the paper: its colour, the ink on it, a soft shadow on the
/// desk. Set on the sheet's container, so everything on the sheet inherits
/// the ink.
fn paper_style(palette: &Palette) -> cosmic::theme::Container<'static> {
    let (paper, ink, shadow) = (palette.paper, palette.ink, palette.shadow);
    cosmic::theme::Container::custom(move |_theme| iced::widget::container::Style {
        text_color: Some(ink),
        background: Some(iced::Background::Color(paper)),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 2.0.into() },
        shadow: Shadow { color: shadow, offset: Vector::new(0.0, 3.0), blur_radius: 16.0 },
        ..Default::default()
    })
}

/// Sojourner's own first page. Everything on it is either the app's name or
/// a fact recorded in assets/SOURCES.md.
fn title_page<'a>(palette: &Palette) -> Element<'a, Message> {
    let sheet = column![
        text("Sojourner").size(46).font(SERIF),
        space::vertical().height(10),
        container(divider::horizontal::light()).width(160),
        space::vertical().height(18),
        text("The Holy Bible").size(26).font(SERIF),
        space::vertical().height(6),
        text("World English Bible").size(18).font(SERIF),
        space::vertical().height(28),
        text("Updated edition · Public domain · eBible.org").size(13).class(palette.page_muted),
        text("Cross-references from openbible.info, CC BY 4.0").size(13).class(palette.page_muted),
        space::vertical().height(40),
        text("Turn the page with → or the arrow beside it").size(13).class(palette.page_muted),
    ]
    .align_x(iced::Alignment::Center)
    .spacing(0);

    container(sheet)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

// ────────────────────────────────────────────────────────────────────────────
// Drawing the sheet
// ────────────────────────────────────────────────────────────────────────────

/// A paragraph drawn: its pieces as spans, with the colours and links the
/// measurer left out, the verse being read aloud highlighted.
fn par_view<'a>(par: &Par, g: &Geometry, palette: &Palette, spoken: Option<u32>) -> Rich<'a, PageLink, Message, cosmic::Theme, cosmic::Renderer> {
    let spans: Vec<Span<'a, PageLink, Font>> = par
        .pieces
        .iter()
        .map(|piece| {
            let mut s = shaped_span(piece.text.clone(), piece, g);
            match piece.ink {
                Ink::Body => {}
                Ink::Muted => s = s.color(palette.page_muted),
                Ink::Red => s = s.color(palette.red_letter),
                Ink::Accent => s = s.color(palette.accent),
            }
            if let Some(link) = piece.link {
                s = s.link(link);
            }
            if spoken.is_some() && piece.verse == spoken {
                s = s.background(palette.spoken);
            }
            s
        })
        .collect();
    rich_text(spans)
        .size(g.size)
        .line_height(LineHeight::Absolute(Pixels(g.line)))
        .font(g.font)
        .align_x(g.align)
        .width(Length::Fill)
        .on_link_click(Message::Follow)
}

/// A paragraph on the sheet, with its air and indent. The first on a sheet
/// gets no air above it.
fn par_element<'a>(par: &Par, body: f32, palette: &Palette, spoken: Option<u32>, first: bool) -> Element<'a, Message> {
    let g = geometry(par.shape, body);
    let above = if first { 0.0 } else { g.above };
    if is_air(par) {
        return space::vertical().height(above).into();
    }
    container(par_view(par, &g, palette, spoken))
        .padding([above, 0.0, g.below, g.indent])
        .into()
}

// ────────────────────────────────────────────────────────────────────────────
// Contents sidebar
// ────────────────────────────────────────────────────────────────────────────

/// Width of the Contents sidebar.
const CONTENTS_WIDTH: f32 = 300.0;

impl Sojourner {
    /// The shelf as a list: section headings, one row per file, and the
    /// chapter numbers of whichever book is unfolded. Sits to the left of
    /// the page.
    fn contents(&self) -> Element<'_, Message> {
        let Some(lib) = &self.library else {
            return text("").into();
        };
        let palette = Palette::current();
        let mut list = column![].spacing(2);
        let mut last_section: Option<Section> = None;

        for (slot_ix, slot) in self.shelf.iter().enumerate() {
            let here = slot_ix == self.at.slot;
            match *slot {
                Slot::Title => {
                    list = list.push(
                        button::text(if here { "▸ Title page" } else { "Title page" })
                            .on_press(Message::GoTo(Position { slot: slot_ix, page: 0, leaf: 0 })),
                    );
                }
                Slot::Book(i) => {
                    let book = &lib.bible.books[i];
                    if last_section != Some(book.section) {
                        last_section = Some(book.section);
                        list = list.push(space::vertical().height(12));
                        list = list.push(text(section_name(book.section)).size(12).class(palette.muted));
                    }
                    let name = if here {
                        format!("▸ {}", book.names.short)
                    } else {
                        book.names.short.clone()
                    };
                    if book.chapters.is_empty() {
                        // Preface and glossary: one page, no chapters to unfold.
                        list = list.push(
                            button::text(name).on_press(Message::GoTo(Position { slot: slot_ix, page: 0, leaf: 0 })),
                        );
                    } else {
                        list = list.push(button::text(name).on_press(Message::ExpandBook(slot_ix)));
                        if self.expanded == Some(slot_ix) {
                            let numbers: Vec<Element<'_, Message>> = book
                                .chapters
                                .iter()
                                .enumerate()
                                .map(|(page, chapter)| {
                                    let label = chapter.number.to_string();
                                    let go = Message::GoTo(Position { slot: slot_ix, page, leaf: 0 });
                                    // The chapter being read is the one filled-in button.
                                    if here && page == self.at.page {
                                        button::suggested(label).on_press(go).into()
                                    } else {
                                        button::text(label).on_press(go).into()
                                    }
                                })
                                .collect();
                            list = list.push(container(flex_row(numbers).spacing(2)).padding([2, 0, 6, 12]));
                        }
                    }
                }
            }
        }

        container(scrollable(container(list).padding([16, 12, 24, 12])))
            .width(CONTENTS_WIDTH)
            .height(Length::Fill)
            .into()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Trail panel
// ────────────────────────────────────────────────────────────────────────────

impl Sojourner {
    /// The breadcrumbs, then whatever the last stop is.
    fn trail_panel(&self) -> Element<'_, Message> {
        let Some(lib) = &self.library else {
            return text("").into();
        };
        let palette = Palette::current();
        let mut body = column![].spacing(10);

        // Breadcrumbs: every earlier stop is a link back to it.
        if self.trail.len() > 1 {
            let mut crumbs: Vec<Element<'_, Message>> = Vec::new();
            for (i, stop) in self.trail.iter().enumerate() {
                if i > 0 {
                    crumbs.push(text("›").size(13).class(palette.muted).into());
                }
                if i + 1 == self.trail.len() {
                    crumbs.push(text(self.stop_label(stop)).size(13).into());
                } else {
                    crumbs.push(button::link(self.stop_label(stop)).on_press(Message::Backtrack(i)).into());
                }
            }
            body = body.push(flex_row(crumbs).spacing(4));
            body = body.push(divider::horizontal::light());
        }

        match self.trail.last() {
            None => body = body.push(text("Click a verse number to see its references.").class(palette.muted)),
            Some(Stop::Verses { start, end }) => body = body.push(self.verses_stop(lib, *start, *end, &palette)),
            Some(Stop::Note { at, note }) => body = body.push(self.note_stop(lib, *at, note, &palette)),
            Some(Stop::Xref { at, xref }) => body = body.push(self.xref_stop(lib, *at, xref, &palette)),
        }

        scrollable(container(body).padding([0, 12, 24, 0])).into()
    }

    /// A verse (or range): its text, the translators' own notes on it, a
    /// "Go there" button, then its cross-references most-voted first, each
    /// with the beginning of its text.
    fn verses_stop<'a>(&'a self, lib: &'a Library, start: VerseRef, end: VerseRef, palette: &Palette) -> Element<'a, Message> {
        let mut body = column![].spacing(8);

        // The verse text, block structure kept, with its own footnotes and
        // markers clickable. Set the way the page sets it, a little smaller.
        let mut shown_marker: Option<u32> = None;
        for v in lib.index.between(start, end) {
            let Some(found) = scripture::verse(&lib.bible, *v) else { continue };
            // A bridged verse (15-16) appears once, not twice.
            if shown_marker == Some(found.number.start) {
                continue;
            }
            shown_marker = Some(found.number.start);
            let mut setter = Setter::new(v.book, v.chapter);
            for (kind, inlines) in &found.pieces {
                let shape = Shape::Block(*kind);
                let par = set_content(shape, inlines, kind.is_verse_flow().then_some(&mut setter));
                let g = geometry(shape, PANEL_BODY);
                body = body.push(container(par_view(&par, &g, palette, None)).padding([g.above, 0.0, g.below, g.indent]));
            }
        }

        let go_label = format!("Go to {}", scripture::reference_label(&lib.bible, start, start));
        body = body.push(
            container(
                row![
                    button::standard(go_label).on_press(Message::GoToVerse(start)),
                    button::text("Read from here").on_press(Message::ReadFrom(start)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding([6, 0, 4, 0]),
        );

        // The translators' own cross-references anchored in these verses.
        let mut own: Vec<&CrossRef> = Vec::new();
        for v in lib.index.between(start, end) {
            if let Some(found) = scripture::verse(&lib.bible, *v) {
                for (_, inlines) in &found.pieces {
                    for inline in *inlines {
                        if let Inline::CrossRef(xref) = inline {
                            own.push(xref);
                        }
                    }
                }
            }
        }
        if !own.is_empty() {
            body = body.push(text("Translators' cross-references").size(12).class(palette.muted));
            for xref in own {
                body = body.push(self.targets_row(lib, xref, palette));
            }
        }

        // The web: openbible.info's references, most-voted first.
        let links = lib.refs.from(start);
        let heading = if links.is_empty() {
            "No cross-references for this verse".to_string()
        } else {
            format!("Cross-references ({})", links.len())
        };
        body = body.push(container(text(heading).size(12).class(palette.muted)).padding([8, 0, 0, 0]));

        for link in links {
            let label = scripture::reference_label(&lib.bible, link.target, link.target_end);
            let quote = quote_of(lib, link.target, link.target_end);
            body = body.push(
                column![
                    button::link(label).on_press(Message::Peek(link.target, link.target_end)),
                    container(text(quote).size(14).font(SERIF).class(palette.muted)).padding([0, 0, 4, 12]),
                ]
                .spacing(0),
            );
        }

        body.into()
    }

    /// A footnote on its own: the note's text, and the way back to its verse.
    fn note_stop<'a>(&'a self, lib: &'a Library, at: VerseRef, note: &'a Footnote, palette: &Palette) -> Element<'a, Message> {
        let mut body = column![].spacing(8);
        body = body.push(text(format!("Footnote at {}", note.reference)).size(12).class(palette.muted));
        body = body.push(note_flow(note, palette));
        let go_label = format!("Go to {}", scripture::reference_label(&lib.bible, at, at));
        body = body.push(container(button::standard(go_label).on_press(Message::GoToVerse(at))).padding([6, 0, 0, 0]));
        body.into()
    }

    /// One of the translators' own cross-references on its own: their
    /// wording, and each target as a link.
    fn xref_stop<'a>(&'a self, lib: &'a Library, at: VerseRef, xref: &'a CrossRef, palette: &Palette) -> Element<'a, Message> {
        let mut body = column![].spacing(8);
        body = body.push(text(format!("Translators' cross-reference at {}", xref.origin)).size(12).class(palette.muted));
        body = body.push(self.targets_row(lib, xref, palette));
        let go_label = format!("Go to {}", scripture::reference_label(&lib.bible, at, at));
        body = body.push(container(button::standard(go_label).on_press(Message::GoToVerse(at))).padding([6, 0, 0, 0]));
        body.into()
    }

    /// The targets of a translators' cross-reference, each as a link. The
    /// translators' own wording is shown only when something in it could
    /// not be resolved, so the reader sees exactly what was written.
    fn targets_row<'a>(&'a self, lib: &'a Library, xref: &'a CrossRef, palette: &Palette) -> Element<'a, Message> {
        let targets = scripture::parse_targets(&lib.bible, &lib.index, &xref.targets);
        let mut items: Vec<Element<'a, Message>> = Vec::with_capacity(targets.len() + 1);
        if targets.iter().any(|t| t.range.is_none()) {
            items.push(text(format!("‡ {}", xref.targets)).size(14).font(SERIF_ITALIC).class(palette.muted).into());
        }
        for target in targets {
            match target.range {
                Some((s, e)) => items.push(button::link(target.label).on_press(Message::Peek(s, e)).into()),
                None => items.push(text(target.label).size(14).into()),
            }
        }
        flex_row(items).spacing(4).into()
    }
}

/// The opening of a referenced passage, for reading under its link.
fn quote_of(lib: &Library, start: VerseRef, end: VerseRef) -> String {
    let mut out = String::new();
    let mut shown_marker: Option<u32> = None;
    for v in lib.index.between(start, end) {
        let Some(found) = scripture::verse(&lib.bible, *v) else { continue };
        if shown_marker == Some(found.number.start) {
            continue;
        }
        shown_marker = Some(found.number.start);
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&found.plain());
        if out.chars().count() > QUOTE_CHARS {
            break;
        }
    }
    if out.chars().count() > QUOTE_CHARS {
        let cut: String = out.chars().take(QUOTE_CHARS).collect();
        format!("{}…", cut.trim_end())
    } else {
        out
    }
}

/// A footnote's runs as rich text: `\fq` and `\fqa` in italic, Hebrew words
/// as they are.
fn note_flow<'a>(note: &'a Footnote, _palette: &Palette) -> Rich<'a, PageLink, Message, cosmic::Theme, cosmic::Renderer> {
    let spans: Vec<Span<'a, PageLink, Font>> = note
        .content
        .iter()
        .map(|run| {
            span(run.text.as_str()).font(match run.style {
                TextStyle::NoteQuote | TextStyle::NoteAlternate | TextStyle::BookName => SERIF_ITALIC,
                TextStyle::NoteLabel => SERIF_BOLD,
                _ => SERIF,
            })
        })
        .collect();
    rich_text(spans)
        .size(PANEL_BODY)
        .line_height(PANEL_LINE)
        .font(SERIF)
        .on_link_click(Message::Follow)
}

// ────────────────────────────────────────────────────────────────────────────
// Colours
// ────────────────────────────────────────────────────────────────────────────

/// The few colours the window uses. The book's are Sojourner's own — the
/// desk, the paper, its ink, the links on it — a warm cream by day and a
/// warm dark by night, chosen by whether the COSMIC theme is light or dark
/// and otherwise the same on every desktop; only the chrome around the book
/// (the panel, the sidebar, the header) follows the theme.
struct Palette {
    /// Verse numbers and note markers: the links on the sheet.
    accent: Color,
    /// Quiet text off the sheet (the panel, the sidebar).
    muted: Color,
    /// The desk the sheet lies on.
    desk: Color,
    /// The sheet.
    paper: Color,
    /// Text on the sheet.
    ink: Color,
    /// Quiet text on the sheet: superscriptions, the running head.
    page_muted: Color,
    /// The sheet's shadow on the desk.
    shadow: Color,
    /// Words of Jesus.
    red_letter: Color,
    /// Behind the verse being read aloud: the accent, faint.
    spoken: Color,
}

impl Palette {
    fn current() -> Self {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let dark = cosmic.is_dark;
        // Night: a warm dark sheet on a darker desk, light ink, links a
        // soft blue. Day: cream on a warm grey desk, near-black ink, links
        // a slate blue. Change them here and nowhere else.
        let (desk, paper, ink, accent, red_letter, shadow) = if dark {
            (
                Color::from_rgb8(0x1B, 0x1A, 0x18),
                Color::from_rgb8(0x26, 0x23, 0x20),
                Color::from_rgb8(0xDC, 0xD6, 0xC8),
                Color::from_rgb8(0x7F, 0xB0, 0xD6),
                Color::from_rgb8(0xD9, 0x6A, 0x62),
                Color { a: 0.5, ..Color::BLACK },
            )
        } else {
            (
                Color::from_rgb8(0xD9, 0xD6, 0xD0),
                Color::from_rgb8(0xFA, 0xF6, 0xEE),
                Color::from_rgb8(0x2B, 0x26, 0x20),
                Color::from_rgb8(0x2F, 0x6F, 0x9F),
                Color::from_rgb8(0xB4, 0x2A, 0x2A),
                Color { a: 0.22, ..Color::BLACK },
            )
        };
        Palette {
            accent,
            muted: cosmic.palette.neutral_7.into(),
            desk,
            paper,
            ink,
            page_muted: Color { a: 0.6, ..ink },
            shadow,
            red_letter,
            spoken: Color { a: 0.22, ..accent },
        }
    }
}

/// The voice thread, as a subscription: started once, alive as long as the
/// window. The first thing out of it is the handle the window sends commands
/// with; everything after is what the voice reports back.
fn voice_thread() -> impl Stream<Item = Message> {
    iced::stream::channel(256, async |mut output| {
        let mut events = output.clone();
        let reader = Reader::spawn(move |event| {
            // A full channel means the window is far behind; dropping a
            // report is better than stalling the voice.
            let _ = events.try_send(Message::Reader(event));
        });
        let _ = output.try_send(Message::ReaderStarted(reader));
        // Keep the channel open for as long as the window lives.
        std::future::pending::<()>().await;
    })
}

fn main() -> cosmic::iced::Result {
    // The book face goes into the text engine before the window opens, so
    // the first sheet is set and drawn in it.
    sojourner::page::load_book_face();
    // Resizing is allowed; a smaller window scrolls the sheet, a larger one
    // gets more desk.
    let settings = cosmic::app::Settings::default().size(window_size());
    cosmic::app::run::<Sojourner>(settings, ())
}
