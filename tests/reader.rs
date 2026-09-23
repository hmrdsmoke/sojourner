// ── Add your header block above ──

//! Proof that the reader keeps its place: with a stand-in voice (silence,
//! sized to the words) and no sound device, it announces each verse in
//! order, skips the wordless ones, pauses and resumes, goes back and skips
//! forward on command, and says when it is done. The real voice and the real
//! speakers are exercised by tests/voice.rs and by listening.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use sojourner::text::VerseRef;
use sojourner::voice::reader::{Command, Event, Output, Reader, Verse};
use sojourner::voice::{Audio, Error, Voice};

/// A voice that "says" each word as a fifth of a second of a quiet tone.
struct Metronome;

impl Voice for Metronome {
    fn name(&self) -> &str {
        "metronome"
    }
    fn speak(&mut self, text: &str) -> Result<Audio, Error> {
        let words = text.split_whitespace().count();
        let n = 22_050 / 5 * words;
        let samples = (0..n).map(|i| ((i as f32) * 0.05).sin() * 0.2).collect();
        Ok(Audio { sample_rate: 22_050, samples })
    }
}

fn verse(n: u32, text: &str) -> Verse {
    Verse { at: VerseRef { book: 42, chapter: 1, verse: n }, text: text.to_string() }
}

/// Wait for the next event, or fail after a while.
fn next(events: &mpsc::Receiver<Event>) -> Event {
    events.recv_timeout(Duration::from_secs(5)).expect("an event within five seconds")
}

fn expect_speaking(events: &mpsc::Receiver<Event>, index: usize) {
    match next(events) {
        Event::Speaking { index: i, at } if i == index => assert_eq!(at.verse as usize, index + 1),
        other => panic!("expected Speaking {{ index: {index} }}, got {other:?}"),
    }
}

#[test]
fn the_reader_keeps_its_place() {
    let (tx, events) = mpsc::channel();
    // Twenty times real time: a four-word verse takes 40 ms.
    let reader = Reader::spawn_test(Box::new(Metronome), Output::Drain { speed: 20.0 }, move |e| {
        let _ = tx.send(e);
    });

    let verses = vec![
        verse(1, "In the beginning God"),   // 0.8 s
        verse(2, "created the heavens"),    // 0.6 s
        verse(3, ""),                       // a footnoted-only verse: nothing to say
        verse(4, "and the earth"),          // 0.6 s
    ];

    // A plain read-through: ready, then every verse with words, then done.
    let started = Instant::now();
    reader.read(verses.clone(), 0);
    assert!(matches!(next(&events), Event::Ready { .. }));
    expect_speaking(&events, 0);
    expect_speaking(&events, 1);
    expect_speaking(&events, 3);
    assert!(matches!(next(&events), Event::Finished));
    // 2.0 s of speech plus three 0.4 s gaps, at 20×: well under a second.
    assert!(started.elapsed() < Duration::from_secs(2), "took {:?}", started.elapsed());

    // Pause holds the place; resume carries on from it.
    reader.read(verses.clone(), 0);
    expect_speaking(&events, 0);
    reader.send(Command::Pause);
    assert!(matches!(next(&events), Event::Paused));
    assert!(
        events.recv_timeout(Duration::from_millis(300)).is_err(),
        "nothing should happen while paused"
    );
    reader.send(Command::Resume);
    assert!(matches!(next(&events), Event::Resumed));
    expect_speaking(&events, 1);

    // Back goes to the previous verse with words; forward skips ahead.
    reader.send(Command::Previous);
    expect_speaking(&events, 0);
    reader.send(Command::Next);
    expect_speaking(&events, 1);
    reader.send(Command::Next);
    expect_speaking(&events, 3);
    // Back over the wordless verse 3 lands on verse 2.
    reader.send(Command::Previous);
    expect_speaking(&events, 1);

    // Stop ends it; a new read starts over from where it is told.
    reader.send(Command::Stop);
    assert!(matches!(next(&events), Event::Stopped));
    reader.read(verses, 3);
    expect_speaking(&events, 3);
    assert!(matches!(next(&events), Event::Finished));
}
