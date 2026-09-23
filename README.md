# Sojourner

A Bible for the COSMIC desktop that reads to you.

Sojourner opens the World English Bible (the publisher's Updated edition)
like a book — a title page, the publisher's preface, every book from Genesis
to the glossary, with the Deuterocanon shelved after the New Testament — and
reads it aloud, verse by verse, with a voice that runs entirely on your
machine. Verse numbers and note markers are links: click one and a panel
opens on that verse and its cross-references, and you can follow a reference
to the next verse and the next, with breadcrumbs back to where you started,
while the page you were on stays put.

It is being built in the open and is not finished. What works today:

- The whole text, every word the publisher shipped: 83 files, 38,058 verses,
  1,956 footnotes, the translators' own cross-references, the Deuterocanon,
  the glossary. Proven word for word against the publisher's own rendering
  (`cargo test`). The divine name is set as the publisher sets it, "LORD"
  and "GOD" in small capitals.
- 344,799 cross-references from openbible.info, every one resolved against
  the text.
- Reading aloud with a local neural voice (Piper's LJ Speech voice):
  Play/Space reads the chapter, the verse being spoken is highlighted and
  kept in view, Shift+Up/Down go back a verse or skip one, and a chapter
  that ends runs on into the next.
- The names of scripture said as the dictionaries say them: a table of 682
  names generated from two sourced pronouncing dictionaries (the CMU
  Pronouncing Dictionary and Chambers's 1908 "Pronouncing Vocabulary of
  Scripture Proper Names"), so "Job" is the man, not the work.
- Keyboard page turning, a Contents sidebar, the trail panel; it opens where
  you left off.

Not yet: true fixed-size page flipping and the look of a book, a Flatpak,
settings, and the 2,900 rarer names neither dictionary has.

## Where the words come from

Sojourner does not author scripture. [`assets/SOURCES.md`](assets/SOURCES.md)
records every piece of text, data and voice it uses — who made it, where it
was obtained, its hash, its license, and what Sojourner does with it — so
that anything on screen can be traced back to its source. Nothing is
changed; where the app reorders (the Deuterocanon) or renumbers (two places
in the cross-reference data, both explained by the translators' footnotes),
that is written down too.

Sojourner's own code is GPL-3.0 (see [`LICENSE`](LICENSE)). The text is
public domain, the cross-references CC BY 4.0, the voice's dataset public
domain — those licenses belong to the data and are recorded with it.

## Building

Rust (edition 2024) and, on Pop!_OS or Ubuntu:

```
sudo apt install cmake clang libclang-dev libsonic-dev libasound2-dev
```

Two files are too large for the repository and are obtained separately;
`assets/SOURCES.md` has the URLs and the hashes they must match.

```
# The voice model (114 MB), beside its committed card and config:
#   assets/voices/en_US-ljspeech-high.onnx
#   from https://huggingface.co/rhasspy/piper-voices, en/en_US/ljspeech/high/
# ONNX Runtime 1.28.0 for Linux x64, extracted under assets/onnxruntime/:
#   https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-x64-1.28.0.tgz

cargo test      # the proofs: text, cross-references, phonemes, the reader, the voice
cargo run
```

Without the model and runtime the app still opens and reads as a book; only
the voice is unavailable, and it says so.

## Name

A sojourner is one who stays a while in a place that is not their home.
