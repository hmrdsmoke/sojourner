// ── Add your header block above ──

//! Reading along: a worker that speaks a list of verses one at a time and
//! says which one it is on, so the page can follow.
//!
//! The worker is its own thread. It owns the voice (loaded the first time
//! it is asked to read, since the model takes a second or two and a couple
//! hundred megabytes), the speakers, and the list of verses it was given.
//! The window talks to it with `Command`s and hears back through `Event`s;
//! neither side ever waits on the other.
//!
//! Each verse is synthesized whole and played whole, with a short silence
//! after it. While one verse plays the next is synthesized, so the voice
//! keeps ahead of itself on any machine where speaking is faster than
//! listening (on the author's it is about ten times faster). Going back or
//! skipping forward drops whatever is playing and starts the chosen verse
//! from its beginning.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::mixer::{Mixer, MixerSource};
use rodio::{ChannelCount, MixerDeviceSink, Player, SampleRate};

use super::{files, piper::PiperVoice, Audio, Voice};
use crate::text::VerseRef;

/// The silence left after each verse, before the next begins.
const GAP: Duration = Duration::from_millis(400);

/// How often the worker looks up from waiting for a command to see whether
/// the verse being played has ended.
const TICK: Duration = Duration::from_millis(30);

/// One verse to be read: where it is, and the words to say.
#[derive(Debug, Clone)]
pub struct Verse {
    pub at: VerseRef,
    pub text: String,
}

/// What the window asks of the reader.
#[derive(Debug)]
pub enum Command {
    /// Read these verses in order, starting at index `from`. Replaces
    /// whatever was being read.
    Read { verses: Vec<Verse>, from: usize },
    Pause,
    Resume,
    /// Start the previous verse over — or this one, if it is the first.
    Previous,
    /// Skip to the next verse.
    Next,
    /// Stop reading and forget the list.
    Stop,
}

/// What the reader tells the window.
#[derive(Debug, Clone)]
pub enum Event {
    /// The voice is being loaded; the first reading waits a moment.
    Loading,
    /// The voice is loaded and the speakers are open.
    Ready { voice: String },
    /// Reading can't happen: the voice files or a sound device are
    /// missing, or the engine failed. The message says what.
    Unavailable(String),
    /// This verse (index into the list that was given) is being spoken.
    Speaking { index: usize, at: VerseRef },
    Paused,
    Resumed,
    /// The last verse of the list has been spoken.
    Finished,
    /// Reading was stopped by `Stop`. (A new `Read` replaces the old one
    /// silently; the window asked for it and knows.)
    Stopped,
}

/// A handle to the reader thread. Cloning it clones the handle, not the
/// reader; dropping the last one lets the thread finish.
#[derive(Debug, Clone)]
pub struct Reader {
    commands: Sender<Command>,
}

/// Where the sound goes.
pub enum Output {
    /// The system's default output device.
    Speakers,
    /// Nowhere: the sound is consumed at `speed` times real time and
    /// discarded. For tests, which have no sound device and no patience.
    Drain { speed: f32 },
}

/// How the worker gets its voice.
enum VoiceSource {
    /// The bundled voice, from the files `files.rs` looks for.
    Files,
    /// A ready-made voice, handed in. For tests.
    Given(Box<dyn Voice>),
}

impl Reader {
    /// Start a reader that speaks with the bundled voice through the
    /// speakers. `on_event` is called on the reader's own thread.
    pub fn spawn(on_event: impl FnMut(Event) + Send + 'static) -> Reader {
        Self::spawn_with(VoiceSource::Files, Output::Speakers, on_event)
    }

    /// Start a reader with a given voice and output. For tests.
    pub fn spawn_test(voice: Box<dyn Voice>, output: Output, on_event: impl FnMut(Event) + Send + 'static) -> Reader {
        Self::spawn_with(VoiceSource::Given(voice), output, on_event)
    }

    fn spawn_with(voice: VoiceSource, output: Output, on_event: impl FnMut(Event) + Send + 'static) -> Reader {
        let (commands, inbox) = mpsc::channel();
        thread::Builder::new()
            .name("sojourner-reader".into())
            .spawn(move || Worker::new(inbox, voice, output, Box::new(on_event)).run())
            .expect("spawn the reader thread");
        Reader { commands }
    }

    pub fn send(&self, command: Command) {
        // The only way this fails is if the thread is gone, and then there
        // is nobody to tell.
        let _ = self.commands.send(command);
    }

    pub fn read(&self, verses: Vec<Verse>, from: usize) {
        self.send(Command::Read { verses, from });
    }
}

// ────────────────────────────────────────────────────────────────────────────
// The worker
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Playing,
    Paused,
}

struct Worker {
    inbox: Receiver<Command>,
    on_event: Box<dyn FnMut(Event) + Send>,
    voice_source: Option<VoiceSource>,
    voice: Option<Box<dyn Voice>>,
    output: Output,
    speakers: Option<Speakers>,
    verses: Vec<Verse>,
    index: usize,
    state: State,
    /// The player for the verse being spoken. Dropping it stops the sound.
    player: Option<Player>,
    /// The next verse, already synthesized, waiting its turn.
    ahead: Option<(usize, Audio)>,
}

/// An open output: the mixer to play into, and whatever keeps it alive.
struct Speakers {
    mixer: Mixer,
    /// The device, when the output is real. Dropping it stops all sound.
    _device: Option<MixerDeviceSink>,
}

impl Worker {
    fn new(inbox: Receiver<Command>, voice: VoiceSource, output: Output, on_event: Box<dyn FnMut(Event) + Send>) -> Self {
        Worker {
            inbox,
            on_event,
            voice_source: Some(voice),
            voice: None,
            output,
            speakers: None,
            verses: Vec::new(),
            index: 0,
            state: State::Idle,
            player: None,
            ahead: None,
        }
    }

    fn emit(&mut self, event: Event) {
        (self.on_event)(event);
    }

    /// The loop: wait for a command, or — while something is playing — look
    /// up every `TICK` to see whether it has ended.
    fn run(mut self) {
        loop {
            let next = if self.state == State::Playing {
                self.inbox.recv_timeout(TICK)
            } else {
                self.inbox.recv().map_err(|_| RecvTimeoutError::Disconnected)
            };
            match next {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => self.tick(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Read { verses, from } => {
                self.player = None;
                self.state = State::Idle;
                if !self.prepare() {
                    return;
                }
                self.verses = verses;
                self.ahead = None;
                self.start(from);
            }
            Command::Pause => {
                if self.state == State::Playing {
                    if let Some(player) = &self.player {
                        player.pause();
                    }
                    self.state = State::Paused;
                    self.emit(Event::Paused);
                }
            }
            Command::Resume => {
                if self.state == State::Paused {
                    if let Some(player) = &self.player {
                        player.play();
                    }
                    self.state = State::Playing;
                    self.emit(Event::Resumed);
                }
            }
            Command::Previous => {
                if self.state != State::Idle {
                    let back = self.verses[..self.index]
                        .iter()
                        .rposition(|v| !v.text.trim().is_empty())
                        .unwrap_or(self.index);
                    self.start(back);
                }
            }
            Command::Next => {
                if self.state != State::Idle {
                    self.start(self.index + 1);
                }
            }
            Command::Stop => {
                if self.state != State::Idle {
                    self.player = None;
                    self.verses.clear();
                    self.ahead = None;
                    self.state = State::Idle;
                    self.emit(Event::Stopped);
                }
            }
        }
    }

    /// While playing: has the verse ended? If so, on to the next. If not,
    /// use the wait to get the next verse ready.
    fn tick(&mut self) {
        let ended = self.player.as_ref().is_none_or(|p| p.empty());
        if ended {
            self.start(self.index + 1);
            return;
        }
        if self.ahead.is_none() {
            if let Some(next) = self.next_spoken(self.index + 1) {
                if let Some(audio) = self.synthesize(next) {
                    self.ahead = Some((next, audio));
                }
            }
        }
    }

    /// The first verse at or after `index` that has words to say. Verses
    /// with none (the footnoted-only ones) are passed over.
    fn next_spoken(&self, index: usize) -> Option<usize> {
        (index..self.verses.len()).find(|&i| !self.verses[i].text.trim().is_empty())
    }

    /// Load the voice and open the speakers, once. False (with the reason
    /// already reported) if either can't be done.
    fn prepare(&mut self) -> bool {
        if self.voice.is_some() && self.speakers.is_some() {
            return true;
        }
        if self.voice.is_none() {
            let source = self.voice_source.take();
            let voice = match source {
                Some(VoiceSource::Given(voice)) => Ok(voice),
                Some(VoiceSource::Files) | None => {
                    self.emit(Event::Loading);
                    load_bundled_voice()
                }
            };
            match voice {
                Ok(voice) => self.voice = Some(voice),
                Err(reason) => {
                    // Try again next time: the files may have been put in place.
                    self.voice_source = Some(VoiceSource::Files);
                    self.emit(Event::Unavailable(reason));
                    return false;
                }
            }
        }
        if self.speakers.is_none() {
            match open_output(&self.output) {
                Ok(speakers) => self.speakers = Some(speakers),
                Err(reason) => {
                    self.emit(Event::Unavailable(reason));
                    return false;
                }
            }
        }
        let name = self.voice.as_ref().map(|v| v.name().to_string()).unwrap_or_default();
        self.emit(Event::Ready { voice: name });
        true
    }

    /// Speak verse `index` from its beginning, skipping verses with no words
    /// (the footnoted-only ones). Past the end: finished.
    fn start(&mut self, index: usize) {
        // Whatever was playing stops now.
        self.player = None;

        let Some(index) = self.next_spoken(index) else {
            self.verses.clear();
            self.ahead = None;
            self.state = State::Idle;
            self.emit(Event::Finished);
            return;
        };

        let audio = match self.ahead.take() {
            Some((ready, audio)) if ready == index => Some(audio),
            _ => self.synthesize(index),
        };
        let Some(audio) = audio else {
            // The engine failed on this verse; the reason was reported.
            self.verses.clear();
            self.state = State::Idle;
            return;
        };

        let Some(speakers) = &self.speakers else { return };
        let player = Player::connect_new(&speakers.mixer);
        player.append(buffer(&audio));
        player.append(silence(audio.sample_rate, GAP));
        player.play();
        self.player = Some(player);

        self.index = index;
        self.state = State::Playing;
        let at = self.verses[index].at;
        self.emit(Event::Speaking { index, at });
    }

    fn synthesize(&mut self, index: usize) -> Option<Audio> {
        let text = self.verses[index].text.clone();
        let voice = self.voice.as_mut()?;
        match voice.speak(&text) {
            Ok(audio) => Some(audio),
            Err(e) => {
                self.emit(Event::Unavailable(format!("{e}")));
                None
            }
        }
    }
}

fn load_bundled_voice() -> Result<Box<dyn Voice>, String> {
    files::init_runtime().map_err(|e| e.to_string())?;
    let (model, config) = files::find_voice().map_err(|e| e.to_string())?;
    let voice = PiperVoice::open(&model, &config).map_err(|e| e.to_string())?;
    Ok(Box::new(voice))
}

fn open_output(output: &Output) -> Result<Speakers, String> {
    match output {
        Output::Speakers => {
            let mut device = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| format!("no sound output: {e}"))?;
            device.log_on_drop(false);
            Ok(Speakers { mixer: device.mixer().clone(), _device: Some(device) })
        }
        Output::Drain { speed } => {
            let rate = SampleRate::new(22_050).unwrap();
            let (mixer, source) = rodio::mixer::mixer(ChannelCount::new(1).unwrap(), rate);
            drain(source, rate, *speed);
            Ok(Speakers { mixer, _device: None })
        }
    }
}

/// A thread that pulls samples from the mixer at `speed` times real time
/// and throws them away, standing in for a sound card.
fn drain(mut source: MixerSource, rate: SampleRate, speed: f32) {
    thread::Builder::new()
        .name("sojourner-drain".into())
        .spawn(move || {
            // Ten milliseconds of audio per step.
            let step = rate.get() as usize / 100;
            let sleep = Duration::from_secs_f32(0.01 / speed);
            loop {
                for _ in 0..step {
                    // None means the mixer has no sources right now: silence.
                    let _ = source.next();
                }
                thread::sleep(sleep);
            }
        })
        .expect("spawn the drain thread");
}

fn buffer(audio: &Audio) -> SamplesBuffer {
    SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(audio.sample_rate.max(1)).unwrap(),
        audio.samples.clone(),
    )
}

fn silence(sample_rate: u32, length: Duration) -> SamplesBuffer {
    let n = (sample_rate as f32 * length.as_secs_f32()) as usize;
    SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(sample_rate.max(1)).unwrap(),
        vec![0.0f32; n],
    )
}
