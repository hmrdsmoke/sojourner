# Sources & Lineage

Sojourner does not author scripture. Every text it displays is someone else's
work, and this file records where each piece came from, under what terms, and
what (if anything) Sojourner changed. If something on screen is wrong, this is
where to start tracing it.

Sojourner's own code is licensed GPL-3.0 (see LICENSE). The licenses recorded
below belong to the data, not the code. Sojourner does not and cannot change
them.

---

## 1. Bible text — World English Bible, Updated edition

| | |
|---|---|
| Work | World English Bible Updated ("WEBU"), full ecumenical book set |
| Publisher | eBible.org — Michael Paul Johnson, editor |
| Home | https://worldenglish.bible — https://ebible.org/engwebu/ |
| Basis, as stated by the publisher | American Standard Version (1901); Biblia Hebraica Stuttgartensia (Old Testament); Greek Majority Text (New Testament); Deuterocanon from the Revised Version Apocrypha and Brenton's Septuagint. The Update is the same text as the Classic edition with the divine name rendered "LORD"/"GOD", typo corrections, and "a small number of language updates" (the publisher's FAQ) |
| File obtained | https://ebible.org/Scriptures/engwebu_usfm.zip |
| Obtained | 2026-09-23, by the author, directly from the publisher |
| Publisher build stamp | "HTML generated with Haiola by eBible.org 22 Sep 2026 from source files dated 22 Sep 2026" (from copr.htm) |
| SHA-256 of the zip | `05d61ed5a91dcae0a66781d71f2f91335c9bf836fc0808d10789b9d51ea19b6f` (3,262,186 bytes) |
| Format | USFM 3, UTF-8. 83 files: front matter, 39 Old Testament books, 15 Deuterocanonical files (some hold several traditional books: Greek Daniel carries Susanna, Bel and the Dragon and the Song of the Three; Baruch carries the Letter of Jeremiah), 27 New Testament books, glossary. The same marker set as the Classic edition; the divine name is plain capitals in the text, not a marker |
| License | Public domain, declared by the publisher. The publisher's statement is kept verbatim as `copr.htm` inside the committed zip |
| Publisher's condition | "World English Bible" is a trademark of eBible.org. The publisher asks that anyone who changes the actual text not call the result the World English Bible |
| Publisher's signing key | `keys.asc` inside the committed zip (PGP, mljohnson.org) |

### What Sojourner does with it

- Shows every word the publisher shipped: all 83 files, including footnotes,
  the translators' cross-references, section titles, Psalm superscriptions,
  the front matter and the glossary. Nothing added, nothing removed.
- Does not change the text. The conversion from USFM to Sojourner's internal
  format is done by code in this repository and is verified (below).
- Shows the divine name as the publisher sets it: "LORD" and "GOD" in
  capitals where the Hebrew has the name (YHWH), "Lord" where it has Adonai,
  so "Lord GOD" and "LORD God" keep the source texts' distinction. The
  publisher's own footnote at each book's first occurrence explains the
  convention ("When rendered in ALL CAPITAL LETTERS, “LORD” or “GOD” is the
  translation of God’s Proper Name (Hebrew “יהוה”, usually pronounced
  Yahweh)"). On the page the capitals are set as small capitals — the first
  letter at body size, the rest smaller — which is only typography; the
  letters are the publisher's. The face the page is set in is section 6.
- Shelves the Deuterocanon after the New Testament, as a labeled section.
  The publisher's file order places these books between the testaments;
  Sojourner reorders for display only. The books themselves are unchanged.
- Preserves the publisher's word-level Strong's tags (`\w ...|strong="..."`)
  in the data but does not display them. The publisher's alignment is
  machine-generated and approximate (e.g. Genesis 1:1 tags "God" with the
  number for "heavens"); it would need its own verification before it could
  be shown.

### History: the Classic edition, 2026-09-22 to 2026-09-23

Sojourner began with the Classic edition (`eng-web_usfm.zip`, 2020 stable
text, SHA-256 `fc578675b0f8fedbc0120920e53e89060fac4d6499270ab99964b913eaad6d36`,
obtained 2026-09-22; its HTML rendering `eng-web_html.zip`, SHA-256
`cf6fec91bc4dc3d18f95faf2da10bc871affae4f9f43f69af3dfd26da0545b63`), which
renders the divine name "Yahweh". The author chose the Classic for that
distinction and, on hearing the text read aloud, found "Yahweh God" and
"Lord Yahweh" read badly; the publisher's FAQ says the same thing about its
own editions ("we are unifying the handling of the Tetragrammaton… to be
the same in all variants and dialects except for the classic WEB"), and
the Updated edition is the publisher's own current text. Both are the
publisher's; nothing was edited to make the change. The Classic files are
in the repository's history.

Measured on 2026-09-23 by parsing both editions and comparing every verse
(38,058 verses, all present in both): 32,213 verses are identical; 5,802
differ only in the name — "Yahweh" → "the LORD" (or "LORD" in address,
"THE LORD" where the Classic had capitals), "Yah" → "the LORD" (24 verses),
"Lord Yahweh" → "Lord GOD", and "Yahweh of Armies" → "the GOD of Armies"
once (Amos 9:5); 43 differ otherwise, and they are: seven language updates
(Genesis 2:5 "in the earth" → "on the earth"; Psalm 68:29 "shall" →
"will"; 1 Maccabees 1:53, 1:56, 1:57 reworded; 2 Maccabees 4:26 and 2
Esdras 4:9 punctuation), five verses where the Update's file lost the space
after a footnote (Greek Esther 1:1 "thingsin", 8:13 "theirbenefactors"; 1
Maccabees 4:40 "trumpets,and"; 1 Esdras 5:8 "Resaias,Eneneus,", 9:27
"andAedias.") — errors in the publisher's file as shipped, kept as shipped
and to be reported to the publisher — and eight New Testament verses that end
with a trailing space (no visible effect). The Update adds 101 footnotes:
the divine-name note above at each book's first occurrence (58), "Hebrew:
יה Yah" where the Classic had "Yah" (43), and a few others; hence 1,956
footnotes and 683,871 word tags against the Classic's 1,855 and 677,690.

### Verification

- Independent witness: TehShrike/world-english-bible, a separate JSON
  conversion of the Classic WEB text (commit `68669ba3be9719ae4d1135b19d9e0b6587b7c356`,
  2025-11-24, https://github.com/TehShrike/world-english-bible). It is not
  shipped with Sojourner and is used only to cross-check Sojourner's own
  conversion. Its package declares no data license, which is one reason it is
  not shipped.
- 2026-09-23: Sojourner's parser (`src/text/usfm.rs`), written for the
  Classic edition, parsed the Updated edition unchanged — the same marker
  set — and its proof (`tests/witness.rs`, run by `cargo test`) was re-run
  on it. The census test asserts every count measured on the raw files:
  83 books, 1,402 chapters, 38,058 verse markers (23,145 OT / 6,955
  Deuterocanon / 7,958 NT), 1,956 footnotes, 363 cross-references,
  683,871 word tags, 23,338 poetry lines, 9,254 paragraphs, 1,070 stanza
  breaks, 139 superscriptions, 6 bridged verse numbers; and that the only
  empty verses in the Protestant canon are the five footnoted ones listed
  below. Every count but footnotes and word tags is the Classic's.
- 2026-09-23: verse-by-verse comparison against the JSON witness: all
  31,103 verses of the 66 Protestant books exist in both, none missing,
  none extra. The witness is a pre-2020 snapshot of the Classic, so it
  proves structure, not wording.
- 2026-09-23: word-for-word proof against the publisher's own rendering of
  the Updated edition. Witness: the HTML edition, generated by the
  publisher's tool (Haiola) from the same source files as the USFM zip, on
  the same day. File https://ebible.org/Scriptures/engwebu_html.zip,
  obtained 2026-09-23 by the author, SHA-256
  `7597dd5bf4004c63a8a207141db512ebece6bb45b4b1a382f033c926bdc3a3be`
  (5,078,763 bytes), kept at `assets/engwebu_html.zip` (not shipped; read
  by the test). Result: all 38,058 verses of all 81 books — Old Testament,
  Deuterocanon and New Testament — have identical words and punctuation in
  Sojourner's parse and in the publisher's rendering. The comparison
  ignores two typographic matters, on both sides equally: whitespace
  between two adjacent quotation marks (the renderer normalizes them to a
  non-breaking space) and whitespace beside an em-dash (the renderer sets
  "Selah" as its own block, Psalm 68:32). Nothing else is ignored. The
  renderer's own counts agree with the parser's as well: 38,058 verse
  spans, the words-of-Jesus spans, the note callers, and the same number of
  blocks of every kind.
- The proof is `cargo test`. With both witness files present it runs four
  checks (census, footnoted-verses, JSON structure, publisher HTML) in
  about five seconds; without them, the census and footnoted-verses checks
  still run against the zip compiled into the binary.
- Text-critical features of the WEB that could be mistaken for missing text,
  each explained by the publisher's own footnote at that verse:
  Luke 17:36, Acts 8:37, Acts 15:34, Acts 24:7 (verses found in the Textus
  Receptus but not the Majority Text; the verse number is kept and the TR
  wording is given in the footnote), and Romans 16:25–27 (the doxology is
  placed after Romans 14:23 per the Majority Text; Romans 14 therefore has
  26 verses and Romans 16 ends at verse 25).

---

## 2. Cross-references — openbible.info

| | |
|---|---|
| Work | Bible cross-reference data from openbible.info (Stephen Smith) |
| Home | https://www.openbible.info/labs/cross-references/ |
| Basis, as stated by the compiler | Primarily the Treasury of Scripture Knowledge (public domain, R. A. Torrey and others, 19th century), seeded with openbible.info's own Topical Bible and Bible-search data; relevance votes contributed by the site's users |
| File obtained | https://a.openbible.info/data/cross-references.zip (contains `cross_references.txt`) |
| Obtained | 2026-09-22, by the author, directly from the compiler's site |
| Snapshot | 2026-09-21, as stamped in the file's own header line (`#www.openbible.info CC-BY 2026-09-21`). The set is regenerated as votes change; this hash pins one snapshot |
| SHA-256 of the zip | `83e9db0a08054ed99848531512729f0190dbbac85416f408f2362b5dc36d421d` |
| Format | Tab-separated text, one reference per line: from-verse, to-verse (single verse or range), votes. 344,799 references, 66 Protestant books, OSIS book abbreviations, KJV/ESV-style verse numbering |
| License | Creative Commons Attribution 4.0 (CC BY 4.0), declared on the compiler's page and in the file header. Attribution: "Cross-reference data from openbible.info, CC BY 4.0" |

### What Sojourner does with it

- Ships it whole, compiled into the binary (`assets/cross-references.zip`),
  and resolves every reference against the parsed WEB text at startup
  (`src/crossrefs.rs`). No reference is discarded.
- Shows a verse's links most-voted first, using the compiler's vote column
  as the relevance order. The votes are the compiler's users' judgment, not
  Sojourner's.
- Renumbers references at exactly two places where the set's numbering
  differs from the WEB's, both explained by the translators' own footnotes:
  Romans 16:25–27 → 14:24–26 (the doxology, placed after 14:23 in the WEB
  per the Majority Text), and 3 John 15 → 14 (the set splits the last verse
  of 3 John; the WEB keeps one verse). 226 references are affected. The
  table is `to_web_numbering` in `src/crossrefs.rs`; nothing else is
  changed.
- Keeps the translators' own 363 cross-references from the WEB text (the
  `\x` notes in the USFM) as a separate, labeled source. They are the
  publisher's, not openbible.info's.

### Verification

- 2026-09-22: `cargo test` (`tests/crossrefs.rs`) asserts that all 344,799
  references resolve to verses that exist in the WEB text (0 unresolved),
  that exactly 226 verse numbers were renumbered by the table above, that
  29,363 verses carry outgoing links, that 18 references are ranges running
  from the end of one book into the next (e.g. 2 Chronicles 36:22 – Ezra
  1:3) and are kept as such, that links are ordered most-voted first, and
  that the renumbering works in both directions (the doxology's own
  outgoing links appear from WEB Romans 14:25). Spot check: the most-voted
  reference for Genesis 1:1 is John 1:1–3.

---

## 3. Text-to-speech voice — LJ Speech, high quality (`en_US-ljspeech-high`)

| | |
|---|---|
| Work | Piper voice "ljspeech (high)": a single-speaker English voice model, US English, female, 22,050 Hz |
| Trained by | Bryce Beattie (the voice card's training note is his, in the first person; his page is https://brycebeattie.com/files/tts/) |
| Distributed by | The Piper voices repository, rhasspy/piper-voices, maintained by Michael Hansen — https://huggingface.co/rhasspy/piper-voices, directory `en/en_US/ljspeech/high/` |
| Files obtained | `en_US-ljspeech-high.onnx` (the model), `en_US-ljspeech-high.onnx.json` (its configuration), `MODEL_CARD` (the voice card), from that directory |
| Obtained | 2026-09-22, by the author, directly from the repository |
| SHA-256, model | `5d4f08ba6a2a48c44592eed3ce56bf85e9de3dd4e20df90541ae68a8310c029a` (114,199,011 bytes) |
| SHA-256, configuration | `7e1f4634af596d83cca997fb7a931ba80b70f8a316a2655ee69c55365e0ace14` (4,970 bytes) |
| SHA-256, voice card | `289f7421072d689a3d91f6c632f486af46f9b8b04417536104a4ea667b8a394b` (513 bytes) |
| Format | Piper voice, `piper_version` 1.0.0: a VITS speech model exported to ONNX, driven by espeak-ng phonemes (espeak voice `en`, 157 phoneme ids), one speaker, 22,050 Hz, quality "high". The configuration's default inference settings are noise 0.667, length 1.0, noise-w 0.333 |
| Training data, as stated by the voice card | The LJ Speech Dataset (Keith Ito and Linda Johnson, 2017): "a public domain speech dataset consisting of 13,100 short audio clips of a single speaker reading passages from 7 non-fiction books" — about 24 hours of Linda Johnson's LibriVox recordings, 2016–17. https://keithito.com/LJ-Speech-Dataset/ |
| Training, as stated by the voice card | "Trained from scratch for 1000 epochs on medium quality settings using the LJ Speech dataset. I reencoded the recordings to a bit rate of 22500 Hz so it would match other voices released for Piper TTS." |
| License, dataset | Public domain. The dataset page: "This dataset is in the public domain in the US (and most likely other countries as well)." The voice card: "License: public domain" |
| License, model files | MIT, declared by the distributing repository for its contents (`license: mit` in the repository's README front matter) |

### What Sojourner does with it

- Reads the text aloud with this voice, one verse at a time, on this
  machine. Nothing is sent anywhere; the voice runs entirely locally.
- Ships the model unchanged. The voice card is kept verbatim at
  `assets/voices/en_US-ljspeech-high.MODEL_CARD`, and the configuration at
  `assets/voices/en_US-ljspeech-high.onnx.json`, both committed. The model
  itself (114 MB) is too large for the repository and is not committed; it is
  obtained from the URL above when the app is built or packaged, and the
  hash above is what it must match. `src/voice/files.rs` says where the app
  looks for it.
- Speaks the words of a verse as printed. Verse numbers, footnote markers
  and cross-reference markers are not spoken. A word set in capitals
  ("LORD", "GOD", "HOLY TO THE LORD") is read as the word: espeak-ng would
  spell a short all-capital word after another capitalized one as an
  acronym ("Lord G-O-D"), so every run of two or more capitals is set to an
  initial capital before espeak-ng sees it (`src/voice/phonemes.rs`,
  `as_words`). Speech only; the page keeps the capitals.

### Verification

- 2026-09-22: `cargo test` (`tests/voice.rs`) opens the model with the
  engine below, speaks the first line of Psalm 23 from the WEB, and checks
  that the result is 22,050 Hz audio of a sensible length (about five
  seconds) with real signal in it. It writes the sound to
  `target/voice-check.wav` for a person to listen to. The test skips,
  saying so, when the model or the runtime library is not present. On the
  first run (a debug build, two cores): model loaded in 2.2 s, 4.9 s of
  speech synthesized in 2.1 s — faster than it plays, which is what reading
  along verse by verse needs.
- The voice was chosen by ear by the author from the English Piper voices,
  with each candidate's card fetched and read for its dataset and license
  first. The chosen voice's dataset is public domain and its model files
  are MIT; nothing about it is licensed for non-commercial use only.

---

## 4. Speech engine

Sojourner does not run the Piper program. It runs the voice model above
through the pieces below, each a separately licensed work. All of them are
build-time dependencies fetched by hash-pinned package tooling (Cargo) except
ONNX Runtime, which is a binary obtained separately.

| Piece | What it does | Version | License | Source |
|---|---|---|---|---|
| Piper (lineage) | Defines the voice format the model above is in, and trained it. Sojourner compiles none of its code | — | Original project rhasspy/piper, MIT, archived by its owner on 2025-10-06 with the notice "Development has moved"; its successor OHF-Voice/piper1-gpl, GPL-3.0 | https://github.com/rhasspy/piper — https://github.com/OHF-Voice/piper1-gpl |
| piper-rs | Rust implementation of Piper inference: reads the configuration, turns a phoneme string into model input, runs the model. Sojourner uses only that; see the note on phonemes below | 0.2.0 (crates.io, published 2026-05-21) | MIT | https://github.com/thewh1teagle/piper-rs |
| espeak-rs, espeak-rs-sys | Rust binding to espeak-ng, which the `-sys` crate builds from a bundled copy of the espeak-ng source. Sojourner calls the `-sys` binding directly (`src/voice/phonemes.rs`) | 0.2.0 | MIT (the binding) | same repository |
| espeak-ng | Turns text into phonemes for the model (the model was trained on its `en` phonemes) | 1.52.0.1, as bundled in espeak-rs-sys 0.2.0 | GPL-3.0-or-later, per the license headers of every source file. The bundled copy omits espeak-ng's top-level COPYING file; the `ucd-tools` component carries its own COPYING (GPL) and COPYING.UCD (Unicode license) | https://github.com/espeak-ng/espeak-ng |
| sonic | Speed-change library that espeak-ng's wave generator calls for its own fast speech rates; Sojourner never calls it, but espeak-ng's build requires it. Taken from the system's `libsonic-dev` package and linked statically by Sojourner's own `build.rs` (see below). When no libsonic is installed, espeak-ng's CMake instead clones this exact commit from GitHub at build time — the one network access in the build, and the reason `libsonic-dev` is required | the system's package (0.2.0 on Pop!_OS 24.04); git commit `fbf75c3d6d846bad3bb3d456cbc5d07d9fd8c104` when fetched | Apache-2.0 (Bill Cox) | https://github.com/waywardgeek/sonic |
| ort, ort-sys | Rust binding to ONNX Runtime. Built with `load-dynamic`, so the runtime library is loaded at startup from a path Sojourner chooses rather than downloaded at build time | 2.0.0-rc.12 (the exact version piper-rs 0.2.0 requires) | MIT OR Apache-2.0 | https://github.com/pykeio/ort |
| rodio | Plays the voice's audio: queues each verse's samples and mixes them into the output stream. Built without its file decoders (Sojourner plays no files) | 0.22 | MIT OR Apache-2.0 | https://github.com/RustAudio/rodio |
| cpal | rodio's way to the sound card: on Linux, ALSA — which on Pop!_OS is PipeWire's ALSA compatibility layer. Building it needs the ALSA headers (`libasound2-dev`) | 0.18 | Apache-2.0 | https://github.com/RustAudio/cpal |
| ONNX Runtime | Runs the model | 1.28.0, Linux x64 release build, git commit `da9b5e364c465de65c49d91e696cd6485270757f` | MIT (Microsoft; the tarball's LICENSE and ThirdPartyNotices.txt) | https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-x64-1.28.0.tgz — SHA-256 `a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407` |

### What Sojourner does with it

- Loads `libonnxruntime.so` from a known place at startup
  (`src/voice/files.rs`: an environment variable, the Flatpak's `/app/lib`,
  or `assets/onnxruntime/*/lib` in a source checkout). Nothing is
  downloaded at build time or at run time. The tarball is extracted into
  `assets/onnxruntime/` for development and is not committed.
- Links espeak-ng statically, built from the bundled source by
  espeak-rs-sys at compile time. This is why building Sojourner needs
  `cmake`, `clang` and `libclang-dev` (for the generated bindings) and
  `libsonic-dev`; playback adds `libasound2-dev`. espeak-ng's CMake finds the system's sonic and records
  the dependency for itself, but a static library carries no link
  information and espeak-rs-sys does not pass it on, so the final link
  would fail with undefined `sonic*` symbols; Sojourner's `build.rs`
  supplies the missing line, linking `libsonic.a` statically so the
  finished binary needs no libsonic.so at run time. Its phoneme data is
  compiled at the same time and found through the path compiled in; a
  packaged build must ship that data directory and point espeak-ng at it,
  and must provide sonic itself, since a packaging build has no network.
- Since espeak-ng is GPL-3.0-or-later and is linked into the binary, the
  binary as a whole is distributed under the GPL — which Sojourner's own
  license already is.
- Makes the phonemes itself, the way Piper's own program does. The voice
  was trained on phoneme strings that keep the clause punctuation of the
  text and a space between clauses, and it synthesizes one sentence at a
  time with a short silence (0.2 s) between sentences. piper-rs's text
  path strips the punctuation and runs the clauses together, which left
  the voice with no pauses and no sentence shape; so Sojourner asks
  espeak-ng for the clauses through espeak-rs-sys, restores the punctuation
  that ended each one, and hands piper-rs one sentence of phonemes at a
  time (`src/voice/phonemes.rs`, `src/voice/piper.rs`).

### Verification

- 2026-09-22: the test in section 3 exercises the whole chain — files
  found, runtime loaded, espeak-ng phonemizing, model run — and passes.
- 2026-09-22: `cargo test` (`tests/phonemes.rs`) checks the phonemes
  against Piper's convention on sample verses — commas, colons and
  semicolons kept with a space after them, sentences split at `.` `?` `!`,
  the publisher's curly quotes, parentheses and apostrophes understood —
  and then runs all 38,029 verses that have words through espeak-ng
  (55,834 sentences) and confirms that every phoneme that comes out is one
  the voice has an id for: none is dropped. The 29 wordless verse numbers
  (Luke 17:36, Acts 8:37, 15:34, 24:7, Romans 16:25, and 24 in Sirach)
  each carry the translators' footnote and nothing to say.
- 2026-09-22: `cargo test` (`tests/reader.rs`) runs the reading-along
  thread with a stand-in voice and no sound device and checks that it
  announces each verse in order, passes over verses with no words, pauses
  and resumes in place, goes back a verse and skips forward on command,
  and reports when it is done. Listening is the test of the real thing.
- 2026-09-22: known limitation, to be addressed as its own piece of work.
  espeak-ng's English rules mispronounce some proper names as spelled:
  it reads "Job" as the word "job", and gets, among others, "Zechariah",
  "Melchizedek", "Gethsemane", "Simeon", "Ecclesiastes", "Manasseh" and
  "Gehenna" wrong; "Yahweh" it reads correctly (YAH-way). Sojourner will
  carry a pronunciation table for the names of scripture, with its source
  recorded here when it is added.

---

## 5. The names of scripture — how the voice says them

espeak-ng reads names by the rules for English spelling and gets many of
them wrong ("Job" as the word *job*, "Zechariah" with a *ch*). Sojourner
carries a table, `assets/names.tsv`, of words as spelled in the text and the
sounds to say them with, in espeak-ng's phoneme mnemonics; every row names
its source here. The words on the page never change; the table only changes
what the voice is told (`src/voice/names.rs`). The table is generated from
the sources below by `tools/names/compose.py` and is not edited by hand.

### Sources

| Id | Work | Obtained | License |
|---|---|---|---|
| `cmudict` | The CMU Pronouncing Dictionary, Carnegie Mellon University — https://github.com/cmusphinx/cmudict, file `cmudict.dict` at `master`, 135,166 lines, SHA-256 `81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22`. American English in ARPAbet. Kept in the repository as `assets/names-source/cmudict-names.txt`: the 693 lines whose word is one of the text's proper names, extracted from that file, under a two-line header naming the extraction (the Classic edition's extraction had two more, "yah" and "yahweh") | 2026-09-23, by the author, from GitHub | BSD 2-clause (Copyright 1993–2015 Carnegie Mellon University); the notice is kept as `assets/names-source/LICENSE.cmudict` |
| `chambers1908` | "Pronouncing Vocabulary of Scripture Proper Names", an appendix of *Chambers's Twentieth Century Dictionary of the English Language*, edited by Thomas Davidson, W. & R. Chambers, 1908 — 621 entries, "all common Scripture Names except monosyllables and dissyllables, the latter being always accented on the first syllable". The Wikisource transcription: https://en.wikisource.org/wiki/Chambers%27s_Twentieth_Century_Dictionary_1908/Pronouncing_Vocabulary_of_Scripture_Proper_Names, page id 1231542, revision 3598679 of 2012-01-27, fetched as wikitext (`action=raw`), 16,339 bytes, SHA-256 `c13a04d17d0134b6084436843877ef2a70e6eda191d8d9115812e656406442b8`, kept as `assets/names-source/chambers-1908-scripture-names.wikitext` with the revision record beside it | 2026-09-23, by the author, from Wikisource | The dictionary is in the public domain (published 1908; its editor died in 1923). Wikisource's transcription is offered under CC BY-SA 4.0; attribution: Wikisource contributors |

The list of names itself — `assets/names-source/web-names.tsv`, 3,837 words
with counts — is drawn from the text by `cargo run --bin names-list`: every
capitalized word that never appears in lowercase anywhere in the text, plus
"Job" (which the heuristic misses because "job" the word occurs twice).
Regenerated for the Updated edition on 2026-09-23: the Classic's "Yahweh"
and "Yah" are gone, and so is "Aedias", which the publisher's lost space in
1 Esdras 9:27 fused into "andAedias" (section 1); the table came out the
same 682 rows.

### How a pronunciation is chosen

For each name, in this order (the code is `tools/names/compose.py`):

1. **cmudict, if it has the name** — modern American usage, which is what a
   listener expects — provided its entry passes two checks: every consonant
   the spelling has appears in the pronunciation, in order (cmudict's entries
   for rare names are sometimes garbled: its "Melchizedek" has no *l*), and a
   two-syllable name is stressed on its first syllable (Chambers's stated rule
   for scripture names; cmudict's "Gilead" and "Abram" break it and are
   passed over).
2. **Chambers, otherwise** — and Chambers over cmudict when both have the
   name but disagree on its consonants, since Chambers follows the spelling
   ("Ahasuerus": cmudict has a *sh*).
3. A row is written only when the voice would otherwise say the name
   differently: espeak-ng is asked for both renderings and they are compared.
   Every row in the table changes something.

Decided by hand, the only one: "Job" takes cmudict's second entry (the man,
`JH OW1 B`; the first is the work). (While the Classic edition was in use,
"Yahweh" was also left to the voice, whose rendering the author preferred to
cmudict's; the Updated edition does not print the name.)

### Conversion to espeak-ng "en" mnemonics

Both sources are turned into the mnemonics of espeak-ng's `en` voice — the
voice the model was trained with, so the sounds stay in the model's own
convention: British-style vowels, and no *r* before a consonant or at the
end of a word, the way espeak-ng's `en` renders words.

From ARPAbet (`tools/names/cmu.py`):

| ARPAbet | espeak `en` | | ARPAbet | espeak `en` |
|---|---|---|---|---|
| AA | A: | | IY | i: |
| AE | a | | OW | oU |
| AH0 | @ | | OY | OI |
| AH1, AH2 | V | | UH | U |
| AO | O: | | UW | u: |
| AW | aU | | ER1, ER2 | 3: |
| AY | aI | | ER0 | @ |
| EH | E | | ER before a vowel | the above plus r |
| EY | eI | | consonants | as ARPAbet, except CH→tS, JH→dZ, NG→N, SH→S, TH→T, DH→D, ZH→Z, Y→j, HH→h |
| IH | I | | stress | `'` before a syllable marked 1, `,` before one marked 2 |

R after a vowel and before a consonant or the end is dropped and colours the
vowel: A:, a → A:; E, eI → e@; I, i: → i@; O:, oU → O:; U, u: → U@; aI → aI@;
aU → aU@; V → 3:; @ stays.

From Chambers's respelling (`tools/names/chambers.py`): the entry is split at
its hyphens and after its stress mark; a parenthetical respelling is laid
over the syllables it covers (a leading dash: the last syllables, or from the
stressed one when it carries the mark; a trailing dash: the first; a leading
mark: the syllables after the stressed one); the first form is taken where
the entry offers an "or". Then, syllable by syllable: long vowels ā ē ī ō ū
→ eI i: aI oU ju: (u: after l, r, s, z, sh, ch, j); short vowels stressed a e
i o u → a E I 0 V, unstressed → @ (i → I; e, i → i before another vowel; a
final open e, i, o, u → i i oU u:); ai/ay → eI, au → O:, oi → OI, ou/ow → aU,
oo → u:, ee/ea → i:, ew/eu → ju:, æ/ae → i:, ia/io → i@; consonants by their
usual sounds with ch → k, c → k, ç → s, ph → f, th → T, g hard, x → ks (gz
before a stressed vowel), y before a vowel → j; a lone h after a vowel is
silent; a doubled consonant is one sound; r before a consonant or at the end
is dropped and colours the vowel as above; a final silent e is dropped
("-īte"); a plural s after m, n, l, r is z. An entry with no stress mark
("Nin-e-veh") is stressed on its first syllable, as Chambers's dissyllables
are. Two entries the rules cannot read are left to the voice: "Appii Forum"
(two words) and "Higgaion", whose parenthetical does not fit any pattern.

### Coverage, 2026-09-23

Of the text's 3,836 proper names (38,808 occurrences): 682 rows (16,701
occurrences) — 337 from cmudict, 345 from Chambers; 249 more names the voice
already says as the sources do; 58 cmudict entries rejected by the checks
(most of them then covered by Chambers or by the voice); 2,906 names
that neither source has, mostly two-syllable names and
names that occur once, said by espeak-ng's rules. A third source for those,
or the author's ear on the frequent ones, is the next step.

### Verification

- 2026-09-23: `cargo test` (`tests/names.rs`) proves the mechanism — a
  table word becomes `[[phonemes]]` before espeak-ng sees the text,
  possessives folded in (Job’s → `[[dZ'oUbz]]`, Moses’s → …Iz, Lot’s → …s),
  words that merely begin with a table word untouched, malformed tables
  refused — and the whole pipeline: "Then Job answered Yahweh." phonemizes
  to `ðˈɛn dʒˈəʊb ˈansəd jˈɑːweɪ.` where the bare rules gave `dʒˈɒb`. It
  pins the counts above (re-pinned for the Updated edition), checks that
  every row's word occurs in the text,
  and that every row changes what the voice says and none comes out spelled
  letter by letter.
- The conversion from Chambers was checked against cmudict on the 134 names
  both have, after the rules above were settled: their consonants agree on
  all but eight, each a known difference (cmudict's garbled "Melchizedek"
  and "Ahasuerus"; *s* against *z* in "Artemas", "Salamis", "Syracuse",
  "Methuselah"; *sh* against *s* or *zh* in "Dionysius", "Persia"), and the
  stressed syllable differs on about fifteen where the two traditions differ
  (Chambers "E-lī′sha" and "Del′i-lah", cmudict "EL-i-sha" and "de-LY-lah").
  Where they differ, the order above decides; the author's ear can overrule
  it, as a third source.
- espeak-ng honors `[[ ]]` only with its `option_phoneme_input` global set,
  which only `espeak_Synth` does; Sojourner sets it once after initializing
  (`src/voice/phonemes.rs`). An internal of espeak-ng 1.52 as bundled by
  espeak-rs-sys 0.2.0; the tests above will notice if an update changes it.

---

## 6. Typeface — Gentium Book Plus

| | |
|---|---|
| Work | Gentium Plus, version 6.200 (1 February 2023): the "Gentium Book Plus" family — the same design at a slightly heavier weight, which the designers offer for smaller sizes and screens. Regular, Italic and Bold are used |
| Designer / publisher | SIL International; Gentium was designed by Victor Gaultney and is developed by SIL's Writing Systems Technology team (the FONTLOG's "SIL WSTech Team"). https://software.sil.org/gentium/ |
| File obtained | https://software.sil.org/downloads/r/gentium/GentiumPlus-6.200.zip (10,935,378 bytes) |
| Obtained | 2026-09-23, by the author, directly from the publisher |
| SHA-256 of the zip | `9b21103b79961149b6508791572acb3b2fe7eb621474c57d5e4ee37e76d7b073` |
| Files kept, unchanged, in `assets/fonts/` | `GentiumBookPlus-Regular.ttf` `298d3e2d2cdf0460d27151de50e2a5764d4de4921f5a6fc254ed5aeda6890f1e` (875,136 bytes); `GentiumBookPlus-Italic.ttf` `ab3d2755ad7e43ca680d4b527cc126a0099fb19d659464b5ea6e1f8b537828ad` (951,280 bytes); `GentiumBookPlus-Bold.ttf` `bad0c69e4452a2c34d66754ad56a0f0c12b043cb40c44216337439eeae500831` (891,340 bytes); the publisher's `OFL.txt` (`d8d18e5a…cf63`) and `FONTLOG.txt` (`11aacc79…390f`) from the same zip |
| Format | TrueType, with OpenType and Graphite smart-font tables; the files declare version 6.200 and weights 500 (Regular, Italic) and 800 (Bold) |
| License | SIL Open Font License, version 1.1, with Reserved Font Names "Gentium" and "SIL". Copyright (c) 2003–2023 SIL International. The OFL allows the fonts to be bundled with and redistributed in software as long as they are not sold by themselves and no modified version uses the reserved names; Sojourner bundles them unmodified, under their own names. The OFL is the fonts' license, not Sojourner's; the GPL does not apply to them |
| Why this face | The publisher of the text sets its own HTML edition in Gentium (the USFM and HTML zips carry `gentiumplus.css`), the face was made for scripture and linguistics, and it is open. The Book weight was chosen over the regular by looking at both on screen at the page's size |

### What Sojourner does with it

- Compiles the three files into the binary and loads them into the text
  engine at startup (`src/page.rs`, `load_book_face`), before anything is
  measured or drawn. Every page is measured and drawn in this face and no
  other, so the book sets the same on every machine: the same lines, the
  same sheets, the same page numbers.
- Uses the face as it is. There is no small-capitals feature call (the
  engine offers none); "LORD" is set as a large capital followed by
  smaller capitals, as section 1 says.
- Does not use Gentium's bold italic, or the lighter Gentium Plus family
  in the same release.

### Verification

- 2026-09-23: the three files were taken from the release zip above and
  compared byte for byte with Debian's packaging of the same release
  (`fonts-sil-gentiumplus` 6.200-1, which repackages the publisher's
  zip): identical.
- `cargo test` (`tests/pages.rs`) hashes the three files compiled into the
  binary and holds them to the hashes above, and — because the face is
  fixed — pins the typesetting itself: at the default 20 px, the whole
  text is 4,668 sheets with 1,743 paragraphs split, Psalm 119 the longest
  at 16; Genesis 1 is 2, 3 and 5 sheets at 14, 20 and 26 px. Any change
  to the face, the engine or the rules shows up as a changed count.
