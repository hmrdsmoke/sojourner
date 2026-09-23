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
//! zip, in Sojourner's shelf order. Each chapter is one page, laid out from
//! the parsed blocks. Pages turn with the header buttons (Previous on the
//! left, Next on the right, the title between them) or the arrow keys, and
//! a Contents sidebar opens on the left to jump anywhere.
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
//! highlighted and the page keeps it in view; ⏮ and ⏭ (Shift+Up/Down) go
//! back a verse or skip one, Pause holds the place, and a chapter that ends
//! while its page is showing runs on into the next. "Read from here" in the
//! trail panel starts at any verse.
//!
//! True fixed-size page flipping (a screenful per page) is a later step; for
//! now a long chapter scrolls.

use std::sync::Arc;

use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::app::{Core, Task};
use cosmic::iced::core::text::LineHeight;
use cosmic::iced::keyboard::{self, key::Named};
use cosmic::iced::futures::Stream;
use cosmic::iced::widget::scrollable::{scroll_by, scroll_to, snap_to, AbsoluteOffset, RelativeOffset};
use cosmic::iced::widget::text::{Rich, Span};
use cosmic::iced::widget::{rich_text, span, Id};
use cosmic::iced::{self, Color, Event, Font, Length, Pixels, Size, Subscription};
use cosmic::widget::{button, column, container, divider, flex_row, icon, row, scrollable, space, text};
use cosmic::{Application, ApplicationExt, Element};

// The library's module is also called `text`, which collides with the text
// *widget*; call the library side `scripture` in this file.
use sojourner::text as scripture;
use sojourner::text::{Block, BlockKind, Book, CrossRef, Footnote, Inline, Section, TextStyle, VerseRef};
use sojourner::voice::reader::{self, Reader, Verse};
use sojourner::Library;

// ────────────────────────────────────────────────────────────────────────────
// Type
// ────────────────────────────────────────────────────────────────────────────

/// A serif face for the page. `Family::Serif` asks the system for its
/// default serif; on Pop!_OS that is usually DejaVu Serif or Noto Serif. A
/// bundled book face can replace this later without touching the layout.
const SERIF: Font = Font {
    family: iced::font::Family::Serif,
    ..Font::DEFAULT
};

const SERIF_ITALIC: Font = Font {
    family: iced::font::Family::Serif,
    style: iced::font::Style::Italic,
    ..Font::DEFAULT
};

const SERIF_BOLD: Font = Font {
    family: iced::font::Family::Serif,
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

/// Reading measure: the text column never grows wider than this, however
/// wide the window is. That single cap is most of the "page" feel.
const MEASURE: f32 = 620.0;

/// Body text size and the size of the small verse numbers set into it.
const BODY: f32 = 18.0;
const VERSE_NUMBER: f32 = 11.0;

/// Every line of body text is this tall, whatever is on it. Given as an
/// absolute height, not a multiple, so a line that happens to start with a
/// small verse number is exactly as tall as its neighbours.
const LINE: LineHeight = LineHeight::Absolute(Pixels(BODY * 1.5));

/// The panel sets text a little smaller than the page.
const PANEL_BODY: f32 = 16.0;
const PANEL_LINE: LineHeight = LineHeight::Absolute(Pixels(PANEL_BODY * 1.5));

/// How much of a referenced verse the panel quotes under its link before
/// trailing off. Clicking the link shows the whole thing.
const QUOTE_CHARS: usize = 260;

/// How far one press of Up or Down scrolls the page: two lines of body text.
const SCROLL_STEP: f32 = BODY * 1.5 * 2.0;

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
    /// Whether the Contents sidebar is open on the left.
    contents_open: bool,
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

/// A place on the shelf: which slot, and which page of it. For a book of
/// scripture the page is the chapter (0-based); the title page, preface and
/// glossary each have a single page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    slot: usize,
    page: usize,
}

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
    /// Scroll the page by this many pixels (negative is up).
    ScrollPage(f32),
    /// Open or close the Contents sidebar.
    ToggleContents,
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

/// The one scrollable on the page, so a page turn can put it back to the top.
fn page_scroll_id() -> Id {
    Id::new("sojourner-page")
}

impl Sojourner {
    /// How many pages a slot has.
    fn pages(&self, slot: usize) -> usize {
        match (self.shelf[slot], &self.library) {
            (Slot::Title, _) => 1,
            (Slot::Book(i), Some(lib)) => lib.bible.books[i].chapters.len().max(1),
            (Slot::Book(_), None) => 1,
        }
    }

    /// Turn one page forward or back, crossing from slot to slot along the
    /// shelf and stopping at either cover.
    fn turn(&mut self, forward: bool) {
        if self.library.is_none() {
            return;
        }
        let at = self.at;
        self.at = if forward {
            if at.page + 1 < self.pages(at.slot) {
                Position { slot: at.slot, page: at.page + 1 }
            } else if at.slot + 1 < self.shelf.len() {
                Position { slot: at.slot + 1, page: 0 }
            } else {
                at
            }
        } else if at.page > 0 {
            Position { slot: at.slot, page: at.page - 1 }
        } else if at.slot > 0 {
            Position { slot: at.slot - 1, page: self.pages(at.slot - 1) - 1 }
        } else {
            at
        };
    }

    /// The page a verse is on.
    fn position_of(&self, verse: VerseRef) -> Option<Position> {
        let lib = self.library.as_ref()?;
        let slot = self.shelf.iter().position(|s| *s == Slot::Book(usize::from(verse.book)))?;
        let book = &lib.bible.books[usize::from(verse.book)];
        let page = book.chapters.iter().position(|c| c.number == verse.chapter)?;
        Some(Position { slot, page })
    }

    /// A page turn or a jump: scroll the page back to its top.
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

    /// Keep the spoken verse in view: scroll the page to the verse's share
    /// of the chapter. A verse a third of the way through the chapter shows
    /// about a third of the way down the window — exactly right when every
    /// verse is the same height, near enough when they aren't.
    fn follow_along(&self) -> Task<Message> {
        let Some(reading) = &self.reading else { return Task::none() };
        if self.spoken_on_page().is_none() {
            return Task::none();
        }
        let last = reading.verses.len().saturating_sub(1).max(1) as f32;
        let y = (reading.index as f32 / last).clamp(0.0, 1.0);
        snap_to(page_scroll_id(), RelativeOffset { x: 0.0, y }.into())
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

    const APP_ID: &'static str = "io.github.smokethehmrd.Sojourner";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.header_title = "Sojourner".to_string();
        let mut app = Sojourner {
            core,
            library: None,
            error: None,
            shelf: Vec::new(),
            at: Position { slot: 0, page: 0 },
            contents_open: false,
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

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Loaded(Ok(library)) => {
                self.shelf = shelf_order(&library);
                self.at = Position { slot: 0, page: 0 };
                self.library = Some(library);
                Task::none()
            }
            Message::Loaded(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::NextPage => {
                self.turn(true);
                Self::back_to_top()
            }
            Message::PreviousPage => {
                self.turn(false);
                Self::back_to_top()
            }
            Message::ScrollPage(by) => scroll_by(page_scroll_id(), AbsoluteOffset { x: 0.0, y: by }),
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
                // The sidebar stays open: turning through chapters from it
                // is what it is for.
                self.at = position;
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
                        // Nothing is being read: read the chapter on the page,
                        // from its first verse.
                        let Slot::Book(i) = self.shelf[self.at.slot] else { return Task::none() };
                        let Some(lib) = &self.library else { return Task::none() };
                        let Some(chapter) = lib.bible.books[i].chapters.get(self.at.page) else { return Task::none() };
                        let first = VerseRef { book: i as u8, chapter: chapter.number, verse: 1 };
                        self.start_reading(first);
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

    /// Left/Right (and PageUp/PageDown) turn pages; Up/Down scroll the one
    /// you're on; Space plays or pauses; Shift+Up/Down go back a verse or
    /// skip one. Only key presses no widget claimed are considered, so
    /// typing in a search box later won't turn pages. The second
    /// subscription is the voice thread, which lives as long as the window.
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
        Subscription::batch([keys, Subscription::run(voice_thread)])
    }

    /// The Contents toggle and Previous sit at the left of the header bar,
    /// Next at the right, with the title between them, so the page itself
    /// stays nothing but text.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        if self.library.is_none() {
            return Vec::new();
        }
        vec![
            row![
                button::icon(icon::from_name("open-menu-symbolic")).on_press(Message::ToggleContents),
                button::standard("‹ Previous").on_press(Message::PreviousPage),
            ]
            .spacing(8)
            .into(),
        ]
    }

    /// The reading controls and Next sit at the right. Play shows on any
    /// chapter page; the rest only while something is being read.
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
        vec![
            row![controls, button::standard("Next ›").on_press(Message::NextPage)]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into(),
        ]
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
        let Some(lib) = &self.library else {
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
        };

        let palette = Palette::current();
        let page: Element<'_, Message> = match self.shelf[self.at.slot] {
            Slot::Title => {
                let title = title_page(&palette);
                return if self.contents_open {
                    row![self.contents(), divider::vertical::light(), title]
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                } else {
                    title
                };
            }
            Slot::Book(i) => book_page(&lib.bible.books[i], i as u8, self.at.page, self.spoken_on_page(), &palette),
        };

        // The measure cap plus centering is what turns a window into a page:
        // widen the window and the margins grow, not the line length.
        let sheet = container(page).max_width(MEASURE).padding([24, 32]);
        let mut page_area = column![].width(Length::Fill).height(Length::Fill);
        if let Some(notice) = &self.voice_notice {
            // A word about the voice, above the page, only while there is
            // something to say.
            page_area = page_area.push(
                container(text(notice.as_str()).size(13).class(palette.muted))
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .padding([6, 0, 0, 0]),
            );
        }
        let page_area = page_area.push(
            container(scrollable(sheet).id(page_scroll_id()))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        );

        if self.contents_open {
            row![self.contents(), divider::vertical::light(), page_area]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            page_area.into()
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Pages
// ────────────────────────────────────────────────────────────────────────────

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
        text("Classic edition · Public domain · eBible.org").size(13).class(palette.muted),
        text("Cross-references from openbible.info, CC BY 4.0").size(13).class(palette.muted),
        space::vertical().height(40),
        text("Turn the page with → or Next").size(13).class(palette.muted),
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

/// One page of a publisher's file: a chapter of scripture, or the whole of
/// a chapterless file (the preface, the glossary).
fn book_page<'a>(book: &'a Book, book_ix: u8, page: usize, spoken: Option<u32>, palette: &Palette) -> Element<'a, Message> {
    let mut sheet = column![].spacing(0);

    // The lead-in lines ("The First Book of Moses," / "Commonly Called")
    // are set small and italic; the name itself ("Genesis") upright and
    // large, the way a printed Bible does it.
    if page == 0 && !book.names.title_lines.is_empty() {
        for (i, line) in book.names.title_lines.iter().enumerate() {
            let last = i + 1 == book.names.title_lines.len();
            sheet = sheet.push(if last {
                text(line.as_str()).size(34).font(SERIF)
            } else {
                text(line.as_str()).size(16).font(SERIF_ITALIC).class(palette.muted)
            });
        }
        sheet = sheet.push(space::vertical().height(if book.chapters.is_empty() { 18 } else { 26 }));
    }

    let (blocks, cursor): (&[Block], Option<LinkCursor>) = if book.chapters.is_empty() {
        (&book.intro, None)
    } else {
        let chapter = &book.chapters[page];
        // "John 3", or "Psalm 23" where the publisher gave a chapter label.
        let label = book.chapter_label.as_deref().unwrap_or(&book.names.short);
        let number = chapter
            .published_number
            .clone()
            .unwrap_or_else(|| chapter.number.to_string());
        sheet = sheet.push(text(format!("{label} {number}")).size(30).font(SERIF));
        sheet = sheet.push(space::vertical().height(14));
        (&chapter.blocks, Some(LinkCursor::new(book_ix, chapter.number).speaking(spoken)))
    };

    let mut cursor = cursor;
    for block in blocks {
        sheet = sheet.push(render_block(block, palette, BODY, LINE, cursor.as_mut()));
    }
    sheet.into()
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
                            .on_press(Message::GoTo(Position { slot: slot_ix, page: 0 })),
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
                            button::text(name).on_press(Message::GoTo(Position { slot: slot_ix, page: 0 })),
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
                                    let go = Message::GoTo(Position { slot: slot_ix, page });
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
        // markers clickable.
        let mut shown_marker: Option<u32> = None;
        for v in lib.index.between(start, end) {
            let Some(found) = scripture::verse(&lib.bible, *v) else { continue };
            // A bridged verse (15-16) appears once, not twice.
            if shown_marker == Some(found.number.start) {
                continue;
            }
            shown_marker = Some(found.number.start);
            let mut cursor = LinkCursor::new(v.book, v.chapter);
            for (kind, inlines) in &found.pieces {
                body = body.push(render_content(*kind, inlines, palette, PANEL_BODY, PANEL_LINE, Some(&mut cursor)));
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

// ────────────────────────────────────────────────────────────────────────────
// Rendering the block model
// ────────────────────────────────────────────────────────────────────────────

/// The few colours the page uses, taken from the active COSMIC theme so the
/// page follows light and dark mode.
struct Palette {
    /// Verse numbers and note markers.
    accent: Color,
    /// Superscriptions, speaker labels, small print: present but quieter.
    muted: Color,
    /// Words of Jesus.
    red_letter: Color,
    /// Behind the verse being read aloud: the accent, faint.
    spoken: Color,
}

impl Palette {
    fn current() -> Self {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();
        let accent: Color = cosmic.accent_color().into();
        Palette {
            accent,
            muted: cosmic.palette.neutral_7.into(),
            red_letter: Color::from_rgb(0.72, 0.16, 0.16),
            spoken: Color { a: 0.22, ..accent },
        }
    }
}

/// Which verse the renderer is inside, as it walks a chapter's blocks, so
/// each verse number, footnote marker and cross-reference marker can carry
/// the right link. Footnotes are numbered within their verse, in order.
struct LinkCursor {
    book: u8,
    chapter: u32,
    current: Option<VerseRef>,
    notes: usize,
    xrefs: usize,
    /// The verse being read aloud on this page, if any: its words are
    /// highlighted as the renderer passes through it.
    speaking: Option<u32>,
}

impl LinkCursor {
    fn new(book: u8, chapter: u32) -> Self {
        LinkCursor { book, chapter, current: None, notes: 0, xrefs: 0, speaking: None }
    }

    fn speaking(mut self, verse: Option<u32>) -> Self {
        self.speaking = verse;
        self
    }

    /// Is the renderer inside the verse being read aloud?
    fn in_spoken(&self) -> bool {
        self.speaking.is_some() && self.current.map(|v| v.verse) == self.speaking
    }
}

/// One block of a page, as a widget.
fn render_block<'a>(
    block: &'a Block,
    palette: &Palette,
    size: f32,
    line: LineHeight,
    cursor: Option<&mut LinkCursor>,
) -> Element<'a, Message> {
    // Links only make sense in the verse flow: a footnote in a superscription
    // belongs to no verse.
    let cursor = if block.kind.is_verse_flow() { cursor } else { None };
    render_content(block.kind, &block.content, palette, size, line, cursor)
}

/// A run of inlines laid out as a block of the given kind.
fn render_content<'a>(
    kind: BlockKind,
    content: &'a [Inline],
    palette: &Palette,
    size: f32,
    line: LineHeight,
    cursor: Option<&mut LinkCursor>,
) -> Element<'a, Message> {
    let text_of = |font: Font, cursor: Option<&mut LinkCursor>| flow(content, palette, font, size, line, cursor);

    match kind {
        // A stanza break: just air.
        BlockKind::Blank => space::vertical().height(12).into(),

        // Prose. Paragraph spacing above; the text wraps to the measure.
        BlockKind::Paragraph | BlockKind::NoBreak | BlockKind::IntroParagraph => {
            container(text_of(SERIF, cursor)).padding([8, 0, 0, 0]).into()
        }
        BlockKind::Flush | BlockKind::IndentedFlush => container(text_of(SERIF, cursor)).padding([4, 0, 0, 0]).into(),
        BlockKind::Indented | BlockKind::ListItem | BlockKind::IntroListItem => {
            container(text_of(SERIF, cursor)).padding([4, 0, 0, 24]).into()
        }
        BlockKind::Centered => container(text_of(SERIF, cursor).center())
            .width(Length::Fill)
            .padding([8, 0, 0, 0])
            .into(),

        // Poetry: one line per block, hanging indent by level, no paragraph
        // spacing so the lines read as verse.
        BlockKind::Poetry(level) => {
            let indent = 16 + 24 * (u16::from(level).saturating_sub(1));
            container(text_of(SERIF, cursor)).padding([0, 0, 0, indent]).into()
        }

        // A Psalm's superscription: scripture, set quietly in italic.
        BlockKind::Superscription => container(text_of(SERIF_ITALIC, cursor).class(palette.muted))
            .padding([6, 0, 10, 0])
            .into(),

        // Speaker labels (Song of Songs), section headings.
        BlockKind::Speaker => container(text_of(SERIF_ITALIC, cursor).class(palette.muted))
            .padding([10, 0, 2, 0])
            .into(),
        BlockKind::Heading | BlockKind::IntroHeading => container(text_of(SERIF_BOLD, cursor)).padding([14, 0, 4, 0]).into(),
        BlockKind::MajorHeading => container(text_of(SERIF_BOLD, cursor).size(size + 4.0))
            .padding([18, 0, 6, 0])
            .into(),
    }
}

/// A block's inlines as one wrapped run of rich text: small accent-coloured
/// verse numbers set into the words, footnote and cross-reference markers
/// where the publisher anchored them, words of Jesus in red. With a cursor,
/// the numbers and markers are links.
fn flow<'a>(
    content: &'a [Inline],
    palette: &Palette,
    font: Font,
    size: f32,
    line: LineHeight,
    mut cursor: Option<&mut LinkCursor>,
) -> Rich<'a, PageLink, Message, cosmic::Theme, cosmic::Renderer> {
    let mut spans: Vec<Span<'a, PageLink, Font>> = Vec::with_capacity(content.len() * 2);

    for inline in content {
        match inline {
            Inline::Verse(n) => {
                let label = if n.end > n.start {
                    format!("{}-{}", n.start, n.end)
                } else {
                    n.start.to_string()
                };
                let mut s = span(label).size(VERSE_NUMBER).color(palette.accent).font(Font::DEFAULT);
                if let Some(c) = cursor.as_deref_mut() {
                    let verse = VerseRef { book: c.book, chapter: c.chapter, verse: n.start };
                    c.current = Some(verse);
                    c.notes = 0;
                    c.xrefs = 0;
                    s = s.link(PageLink::Verse(verse));
                    if c.in_spoken() {
                        s = s.background(palette.spoken);
                    }
                }
                spans.push(s);
                spans.push(span("\u{2009}")); // a thin space between number and word
            }
            Inline::Text(run) => {
                let mut s = span(run.text.as_str()).font(match run.style {
                    TextStyle::Selah | TextStyle::Hebrew | TextStyle::NoteQuote | TextStyle::NoteAlternate => SERIF_ITALIC,
                    TextStyle::Keyword => Font::DEFAULT,
                    _ => font,
                });
                if run.style == TextStyle::WordsOfJesus {
                    s = s.color(palette.red_letter);
                }
                if cursor.as_deref().is_some_and(LinkCursor::in_spoken) {
                    s = s.background(palette.spoken);
                }
                spans.push(s);
            }
            Inline::Footnote(_) => {
                let mut s = span("†").size(VERSE_NUMBER).color(palette.accent).font(Font::DEFAULT);
                if let Some(c) = cursor.as_deref_mut() {
                    if let Some(verse) = c.current {
                        s = s.link(PageLink::Note(verse, c.notes));
                        c.notes += 1;
                    }
                }
                spans.push(s);
            }
            Inline::CrossRef(_) => {
                let mut s = span("‡").size(VERSE_NUMBER).color(palette.accent).font(Font::DEFAULT);
                if let Some(c) = cursor.as_deref_mut() {
                    if let Some(verse) = c.current {
                        s = s.link(PageLink::Xref(verse, c.xrefs));
                        c.xrefs += 1;
                    }
                }
                spans.push(s);
            }
        }
    }

    rich_text(spans)
        .size(size)
        .line_height(line)
        .font(font)
        .on_link_click(Message::Follow)
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
    // A portrait, book-shaped window to start in. Resizing still works; the
    // fixed-page mode that locks this down is a later step.
    let settings = cosmic::app::Settings::default().size(Size::new(760.0, 920.0));
    cosmic::app::run::<Sojourner>(settings, ())
}
