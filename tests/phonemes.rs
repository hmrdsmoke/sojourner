// ── Add your header block above ──

//! Proof that the phonemes reach the voice the way Piper's own program
//! sends them: clause punctuation kept, a space between clauses, one string
//! per sentence, curly quotes and parentheses handled. Needs only espeak-ng,
//! which every build of Sojourner carries; no voice model.

use sojourner::voice::phonemes;

fn say(text: &str) -> Vec<String> {
    phonemes::init(None).unwrap_or_else(|e| panic!("{e}"));
    phonemes::sentences(text, "en").unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_comma_is_kept_and_the_words_stay_apart() {
    let s = say("In the beginning, God created the heavens and the earth.");
    assert_eq!(s, vec!["ɪnðə bɪɡˈɪnɪŋ, ɡˈɒd kɹiːˈeɪtɪd ðə hˈɛvənz and ðɪ ˈɜːθ."]);
}

#[test]
fn sentences_are_split_at_their_ends() {
    let s = say("The LORD is my shepherd: I shall lack nothing. He makes me lie down in green pastures.");
    assert_eq!(s.len(), 2, "{s:?}");
    assert_eq!(s[0], "ðə lˈɔːd ɪz maɪ ʃˈɛpəd: aɪʃˌal lˈak nˈʌθɪŋ.");
    assert_eq!(s[1], "hiː mˌeɪks mˌiː lˈaɪ dˌaʊn ɪn ɡɹˈiːn pˈastʃəz.");

    let s = say("Is it not so? Yes! It is.");
    assert_eq!(s, vec!["ɪz ɪt nˌɒt sˈəʊ?", "jˈɛs!", "ɪt ˈɪz."]);
}

#[test]
fn the_publishers_typography_is_understood() {
    // Curly quotes around speech, a semicolon and a colon inside a sentence,
    // a curly apostrophe in a possessive.
    let s = say("God said, “Let there be light,” and there was light.");
    assert_eq!(s, vec!["ɡˈɒd sˈɛd, lˈɛt ðeəbˈiː lˈaɪt, and ðeəwˌɒz lˈaɪt."]);

    let s = say("the earth; and darkness was on the surface of the deep: and God’s Spirit was hovering over the surface of the waters.");
    assert_eq!(
        s,
        vec!["ðɪ ˈɜːθ; and dˈɑːknəs wɒz ɒnðə sˈɜːfɪs ɒvðə dˈiːp: and ɡˈɒdz spˈɪɹɪt wɒz hˈɒvəɹɪŋ ˌəʊvə ðə sˈɜːfɪs ɒvðə wˈɔːtəz."]
    );

    // A parenthesis inside a clause, and a closing quote after the full stop.
    let s = say("He said to them, “Go into all the world (the whole of it), and preach.”");
    assert_eq!(s, vec!["hiː sˈɛd tə ðˌɛm, ɡˌəʊ ˌɪntʊ ˈɔːl ðə wˈɜːld ðə hˈəʊl ɒv ɪt, and pɹˈiːtʃ."]);

    // A verse that ends without a full stop, as many do.
    let s = say("Blessed is the man who doesn’t walk in the counsel of the wicked, nor stand on the path of sinners;");
    assert_eq!(s, vec!["blˈɛst ɪz ðə mˈan hˌuː dˈʌzənt wˈɔːk ɪnðə kˈaʊnsəl ɒvðə wˈɪkɪd, nˈɔː stˈand ɒnðə pˈaθ ɒv sˈɪnəz;"]);
}

/// Words set in capitals are read as words: "GOD" after "Lord" would
/// otherwise be spelled G-O-D.
#[test]
fn capitals_are_read_as_words() {
    assert_eq!(phonemes::as_words("Thus says the Lord GOD: HOLY TO THE LORD. Job’s LXX"), "Thus says the Lord God: Holy To The Lord. Job’s Lxx");
    let s = say("Thus says the Lord GOD.");
    assert_eq!(s, vec!["ðˈʌs sˈɛz ðə lˈɔːd ɡˈɒd."]);
    let s = say("The LORD is my shepherd; I shall lack nothing.");
    assert_eq!(s, vec!["ðə lˈɔːd ɪz maɪ ʃˈɛpəd; aɪʃˌal lˈak nˈʌθɪŋ."]);
}

#[test]
fn nothing_to_say_is_nothing() {
    assert!(say("").is_empty());
    assert!(say("“ ”").is_empty());
}

/// Every verse of the text goes through, and the phonemes that come out are
/// ones the voice has a number for. The few it hasn't are listed, so a new
/// one is noticed.
#[test]
fn every_verse_phonemizes_into_the_voices_alphabet() {
    use std::collections::BTreeMap;
    let lib = sojourner::Library::load_bundled().unwrap_or_else(|e| panic!("{e}"));
    let config: serde_json::Value = serde_json::from_str(include_str!("../assets/voices/en_US-ljspeech-high.onnx.json"))
        .expect("voice config");
    let map = config["phoneme_id_map"].as_object().expect("phoneme_id_map");

    let started = std::time::Instant::now();
    let mut verses = 0usize;
    let mut sentences = 0usize;
    let mut unknown: BTreeMap<char, usize> = BTreeMap::new();
    for book in &lib.bible.books {
        for (_, _, plain) in sojourner::text::plain_verses(book) {
            if plain.trim().is_empty() {
                continue;
            }
            verses += 1;
            for sentence in say(&plain) {
                sentences += 1;
                for c in sentence.chars() {
                    if !map.contains_key(&c.to_string()) {
                        *unknown.entry(c).or_default() += 1;
                    }
                }
            }
        }
    }
    eprintln!("{verses} verses → {sentences} sentences in {:.1} s; unknown phonemes: {unknown:?}", started.elapsed().as_secs_f32());
    // 38,058 verse markers less the 29 that carry a footnote and no words:
    // Luke 17:36, Acts 8:37, 15:34, 24:7, Romans 16:25, and 24 in Sirach.
    assert_eq!(verses, 38_029);
    assert!(sentences >= verses);
    // Pinned: the characters espeak-ng emits that this voice has no id for.
    let unknown_chars: String = unknown.keys().collect();
    assert_eq!(unknown_chars, "", "unexpected unknown phonemes: {unknown:?}");
}
